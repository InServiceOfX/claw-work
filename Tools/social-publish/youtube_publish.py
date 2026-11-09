#!/usr/bin/env python3
"""Explicit, channel-bound YouTube publishing. Google imports are lazy."""
import argparse
import contextlib
import hashlib
import io
import json
import logging
import os
from pathlib import Path
import re
import stat
import sys
import tempfile

READ = 'https://www.googleapis.com/auth/youtube.readonly'
UPLOAD = 'https://www.googleapis.com/auth/youtube.upload'
MANAGE = 'https://www.googleapis.com/auth/youtube.force-ssl'
SCOPES = {'readonly': [READ], 'upload': [READ, UPLOAD],
          'publish': [READ, UPLOAD, MANAGE]}
CHANNEL = re.compile(r'UC[A-Za-z0-9_-]{22}\Z')
VIDEO = re.compile(r'[A-Za-z0-9_-]{11}\Z')


class Rejected(Exception):
    def __init__(self, code):
        self.code = code
        super().__init__(code)


def require(ok, code):
    if not ok:
        raise Rejected(code)


def check_components(path):
    """Reject symlinks in every existing path component, including ancestors."""
    path = Path(os.path.abspath(os.path.expanduser(str(path))))
    for component in [*reversed(path.parents), path]:
        if component.is_symlink():
            raise Rejected('symlink_rejected')
    return path


def outside_git(path):
    require(not any((p / '.git').exists() for p in [path, *path.parents]),
            'state_inside_git_tree')


def protected(info, directory=False):
    require(info.st_uid == os.getuid(), 'state_wrong_owner')
    require(stat.S_ISDIR(info.st_mode) if directory else stat.S_ISREG(info.st_mode),
            'state_wrong_type')
    if not directory:
        require(info.st_nlink == 1, 'hardlink_rejected')
    require(info.st_mode & 0o077 == 0, 'insecure_permissions')
    require(info.st_mode & 0o700 == (0o700 if directory else 0o600),
            'insecure_permissions')


def read_json(path, private=False):
    path = check_components(path)
    fd = os.open(str(path), os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(fd, 'r', encoding='utf-8') as stream:
        info = os.fstat(stream.fileno())
        require(stat.S_ISREG(info.st_mode), 'input_not_regular_file')
        if private:
            protected(info)
        require(info.st_size <= 1024 * 1024, 'json_too_large')
        return json.load(stream)


def default_state():
    if sys.platform == 'win32':
        base = Path(os.environ.get('LOCALAPPDATA', Path.home() / 'AppData/Local'))
    elif sys.platform == 'darwin':
        base = Path.home() / 'Library/Application Support'
    else:
        base = Path(os.environ.get('XDG_CONFIG_HOME', Path.home() / '.config'))
    return base / 'social-publish'


class State:
    def __init__(self, path):
        require(os.name == 'posix', 'protected_storage_requires_posix')
        self.path = check_components(path)
        outside_git(self.path)
        # Existing ancestors must not permit another user to replace our directory.
        for parent in self.path.parents:
            if parent.exists():
                info = parent.stat()
                require(not (info.st_mode & 0o022) or bool(info.st_mode & stat.S_ISVTX),
                        'insecure_state_parent')
        self.path.mkdir(mode=0o700, parents=True, exist_ok=True)
        protected(self.path.lstat(), directory=True)

    def file(self, name):
        require(re.fullmatch(r'[a-zA-Z0-9_.-]+', name) is not None, 'invalid_state_name')
        path = check_components(self.path / name)
        if path.exists():
            protected(path.lstat())
        return path

    def read(self, name):
        return read_json(self.file(name), private=True)

    def write(self, name, value):
        target = self.file(name)
        fd, tmp = tempfile.mkstemp(prefix='.atomic-', dir=str(self.path))
        try:
            with os.fdopen(fd, 'w', encoding='utf-8') as stream:
                json.dump(value, stream)
                stream.flush()
                os.fsync(stream.fileno())
            self.file(name)  # Recheck existing destination before replacement.
            os.replace(tmp, target)
            directory_fd = os.open(str(self.path), os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
            try:
                os.fsync(directory_fd)
            finally:
                os.close(directory_fd)
        finally:
            if os.path.exists(tmp):
                os.unlink(tmp)

    @contextlib.contextmanager
    def lock(self):
        path = self.file('operation.lock')
        try:
            fd = os.open(str(path), os.O_CREAT | os.O_EXCL | os.O_WRONLY | os.O_NOFOLLOW, 0o600)
        except FileExistsError:
            raise Rejected('state_locked') from None
        os.close(fd)
        try:
            yield
        finally:
            path.unlink()

    def receipt_name(self, channel, digest):
        key = hashlib.sha256((channel + ':' + digest).encode()).hexdigest()
        return 'receipt-' + key + '.json'

    def uploaded(self, channel, video_id):
        for path in self.path.glob('receipt-*.json'):
            item = self.read(path.name)
            if (item.get('channel_id') == channel and item.get('video_id') == video_id
                    and item.get('private_readback') is True):
                return True
        return False


def metadata(value):
    require(isinstance(value, dict), 'invalid_metadata')
    require(set(value) == {'title', 'description', 'categoryId', 'tags', 'madeForKids'},
            'invalid_metadata_fields')
    title, description = value['title'], value['description']
    require(isinstance(title, str) and bool(title.strip()) and len(title) <= 100
            and not any(c in title for c in '<>\x00'), 'invalid_title')
    require(isinstance(description, str) and len(description.encode('utf-8')) <= 5000
            and not any(c in description for c in '<>\x00'), 'invalid_description')
    require(isinstance(value['categoryId'], str)
            and re.fullmatch(r'[1-9][0-9]{0,2}', value['categoryId']) is not None,
            'invalid_category')
    tags = value['tags']
    require(isinstance(tags, list) and all(isinstance(t, str) and t.strip()
            and not any(c in t for c in '<>\x00') for t in tags), 'invalid_tags')
    # Conservatively count separators and the quotes YouTube assigns to spaced tags.
    require(sum(len(t) + (2 if ' ' in t else 0) for t in tags)
            + max(0, len(tags) - 1) <= 500, 'invalid_tags')
    require(type(value['madeForKids']) is bool, 'invalid_made_for_kids')
    return {'snippet': {k: value[k] for k in ('title', 'description', 'categoryId', 'tags')},
            'status': {'privacyStatus': 'private', 'selfDeclaredMadeForKids': value['madeForKids']}}


def identity(api, expected):
    response = api.channels().list(part='id', mine=True).execute(num_retries=0)
    items = response.get('items', [])
    require(not response.get('nextPageToken'), 'channel_identity_mismatch')
    require(len(items) == 1 and items[0].get('id') == expected, 'channel_identity_mismatch')
    return {'channel_id': expected}


def video(api, video_id, expected):
    response = api.videos().list(part='snippet,status,processingDetails', id=video_id).execute(num_retries=0)
    items = response.get('items', [])
    require(len(items) == 1 and items[0].get('id') == video_id, 'video_not_found')
    result = items[0]
    require(result.get('snippet', {}).get('channelId') == expected, 'video_channel_mismatch')
    return result


def summary(item):
    vid = item['id']
    require(isinstance(vid, str) and VIDEO.fullmatch(vid), 'invalid_video_response')
    # Return only known status values; never echo arbitrary provider text.
    status = item.get('status', {})
    processing = item.get('processingDetails', {}).get('processingStatus')
    privacy = status.get('privacyStatus')
    upload = status.get('uploadStatus')
    return {'video_id': vid, 'url': 'https://www.youtube.com/watch?v=' + vid,
            'privacy': privacy if privacy in ('private', 'public', 'unlisted') else 'unknown',
            'upload_status': upload if upload in ('uploaded', 'processed', 'failed', 'rejected', 'deleted') else 'unknown',
            'processing': processing if processing in ('processing', 'succeeded', 'failed', 'terminated') else 'unknown'}


def open_media(path):
    path = check_components(path)
    fd = os.open(str(path), os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    stream = os.fdopen(fd, 'rb')
    try:
        require(stat.S_ISREG(os.fstat(stream.fileno()).st_mode), 'media_not_regular_file')
        digest = hashlib.sha256()
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(block)
        stream.seek(0)
        return stream, digest.hexdigest()
    except BaseException:
        stream.close()
        raise


def build_api(credentials, state=None):
    from googleapiclient.discovery import build
    from google_auth_httplib2 import AuthorizedHttp
    import httplib2
    original_refresh = credentials.refresh

    def persist_refresh(request):
        require(state is not None, 'reauthentication_required')
        original_refresh(request)
        state.write('token.json', json.loads(credentials.to_json()))

    credentials.refresh = persist_refresh
    # Prevent the transport replaying an upload initiation after a 401 response.
    transport = AuthorizedHttp(credentials, http=httplib2.Http(timeout=60), max_refresh_attempts=0)
    return build('youtube', 'v3', http=transport, cache_discovery=False)


def load_api(state, permission):
    from google.oauth2.credentials import Credentials
    from google.auth.transport.requests import Request
    info = state.read('token.json')
    require(set(SCOPES[permission]).issubset(set(info.get('scopes', []))), 'insufficient_oauth_scopes')
    credentials = Credentials.from_authorized_user_info(info)
    if credentials.expired and credentials.refresh_token:
        credentials.refresh(Request())
        state.write('token.json', json.loads(credentials.to_json()))
    require(credentials.valid, 'authentication_required')
    return build_api(credentials, state)


def authorize(args, state):
    # Desktop client config is sensitive too: require a protected external file.
    client_path = check_components(args.client_config)
    outside_git(client_path)
    config = read_json(client_path, private=True)
    require(isinstance(config, dict) and isinstance(config.get('installed'), dict),
            'desktop_client_required')
    installed = config['installed']
    # Prevent untrusted configuration from redirecting OAuth secrets to arbitrary hosts.
    require(installed.get('auth_uri') == 'https://accounts.google.com/o/oauth2/auth'
            and installed.get('token_uri') == 'https://oauth2.googleapis.com/token',
            'invalid_oauth_endpoints')
    from google_auth_oauthlib.flow import InstalledAppFlow
    flow = InstalledAppFlow.from_client_config(config, SCOPES[args.permission])
    credentials = flow.run_local_server(host='127.0.0.1', port=0, open_browser=True,
                                        prompt='select_account consent', access_type='offline',
                                        authorization_prompt_message='', success_message='Authorization received. You may close this window.')
    api = build_api(credentials)
    result = identity(api, args.expected_channel_id)
    state.write('token.json', json.loads(credentials.to_json()))
    return dict(result, authenticated=True, permission=args.permission)


def upload(args, state, api, body, digest, media):
    name = state.receipt_name(args.expected_channel_id, digest)
    require(not state.file(name).exists(), 'duplicate_upload_or_ambiguous_attempt')
    receipt = {'channel_id': args.expected_channel_id, 'sha256': digest,
               'phase': 'reserved', 'private_readback': False}
    state.write(name, receipt)
    # A single resumable initiation and chunk loop. No retries or restart on errors.
    request = api.videos().insert(part='snippet,status', body=body,
                                  media_body=media)
    response = None
    while response is None:
        _, response = request.next_chunk(num_retries=0)
    vid = response.get('id')
    require(isinstance(vid, str) and VIDEO.fullmatch(vid), 'invalid_video_response')
    receipt.update(video_id=vid, phase='accepted')
    state.write(name, receipt)
    item = video(api, vid, args.expected_channel_id)
    require(item.get('status', {}).get('privacyStatus') == 'private', 'upload_not_private')
    receipt.update(private_readback=True, phase='private_readback')
    state.write(name, receipt)
    return summary(item)


def publish(args, state, api):
    require(state.uploaded(args.expected_channel_id, args.video_id), 'private_upload_receipt_required')
    item = video(api, args.video_id, args.expected_channel_id)
    status = item.get('status', {})
    require(status.get('privacyStatus') == 'private', 'video_not_private')
    require(item.get('processingDetails', {}).get('processingStatus') == 'succeeded'
            and status.get('uploadStatus') == 'processed', 'processing_not_succeeded')
    require(not status.get('publishAt'), 'scheduled_video_requires_manual_review')
    # videos.update replaces omitted mutable properties in the selected part.
    # Preserve writable status fields, but never echo read-only fields back.
    writable = ('embeddable', 'license', 'publicStatsViewable',
                'selfDeclaredMadeForKids', 'containsSyntheticMedia')
    updated_status = {key: status[key] for key in writable if key in status}
    updated_status['privacyStatus'] = 'public'
    api.videos().update(part='status', body={'id': args.video_id,
                        'status': updated_status}).execute(num_retries=0)
    result = summary(video(api, args.video_id, args.expected_channel_id))
    require(result['privacy'] == 'public', 'publication_readback_failed')
    return result


class JSONParser(argparse.ArgumentParser):
    def error(self, message):
        raise Rejected('invalid_arguments')


def parser():
    p = JSONParser(description='Channel-bound YouTube CLI; JSON results. Explicit network approval required.')
    sub = p.add_subparsers(dest='command', required=True)
    for command in ('auth', 'channel', 'upload', 'status', 'publish'):
        s = sub.add_parser(command)
        s.add_argument('--state-dir', default=str(default_state()), help='Protected state directory outside every Git tree')
        s.add_argument('--expected-channel-id', required=True)
        s.add_argument('--allow-network', action='store_true', help='Approve Google network access and token refresh/persistence')
        if command == 'auth':
            s.add_argument('--allow-auth', action='store_true', help='Approve browser OAuth and local token storage')
            s.add_argument('--client-config', required=True)
            s.add_argument('--permission', choices=SCOPES, default='readonly',
                           help='readonly: inspect; upload: also upload; publish: also edit/delete-capable force-ssl scope (CLI only publishes)')
        if command == 'upload':
            s.add_argument('--allow-upload', action='store_true')
            s.add_argument('--file', required=True)
            s.add_argument('--metadata', required=True)
        if command in ('status', 'publish'):
            s.add_argument('--video-id', required=True)
        if command == 'publish':
            s.add_argument('--allow-publish', action='store_true')
            s.add_argument('--confirm-video-id', required=True)
    return p


def run(args, api_factory=load_api, media_factory=None):
    require(CHANNEL.fullmatch(args.expected_channel_id) is not None, 'invalid_channel_id')
    require(args.allow_network, 'network_approval_required')
    if args.command == 'auth':
        require(args.allow_auth, 'auth_approval_required')
    if args.command == 'upload':
        require(args.allow_upload, 'upload_approval_required')
        body = metadata(read_json(args.metadata))
    if args.command in ('status', 'publish'):
        require(VIDEO.fullmatch(args.video_id) is not None, 'invalid_video_id')
    if args.command == 'publish':
        require(args.allow_publish, 'publish_approval_required')
        require(args.confirm_video_id == args.video_id, 'video_confirmation_mismatch')
    state = State(args.state_dir)
    with state.lock():
        if args.command == 'auth':
            return authorize(args, state)
        permission = {'upload': 'upload', 'publish': 'publish'}.get(args.command, 'readonly')
        api = api_factory(state, permission)
        result = identity(api, args.expected_channel_id)
        if args.command == 'channel':
            return result
        if args.command == 'status':
            return summary(video(api, args.video_id, args.expected_channel_id))
        if args.command == 'publish':
            return publish(args, state, api)
        stream, digest = open_media(args.file)
        with stream:
            if media_factory is None:
                from googleapiclient.http import MediaIoBaseUpload
                media_factory = lambda source: MediaIoBaseUpload(source, mimetype='application/octet-stream',
                                                                chunksize=8 * 1024 * 1024, resumable=True)
            return upload(args, state, api, body, digest, media_factory(stream))


def main(argv=None):
    try:
        args = parser().parse_args(argv)
        # Third-party libraries may print URLs or log HTTP details, including on errors.
        previous_logging = logging.root.manager.disable
        try:
            logging.disable(sys.maxsize)
            with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                result = run(args)
        finally:
            logging.disable(previous_logging)
        print(json.dumps({'ok': True, 'result': result}))
        return 0
    except Rejected as error:
        print(json.dumps({'ok': False, 'error': error.code}))
    except (Exception, KeyboardInterrupt):
        # Never print provider exceptions, request URLs, paths, OAuth data, or tracebacks.
        print(json.dumps({'ok': False, 'error': 'operation_failed',
                          'guidance': 'For an upload attempt, inspect YouTube Studio before retrying. A reserved receipt blocks duplicates.'}))
    return 1


if __name__ == '__main__':
    sys.exit(main())
