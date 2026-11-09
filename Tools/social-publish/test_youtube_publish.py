"""Offline tests: no Google dependency, credentials, browser, or network."""
import contextlib
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import Mock, patch

import youtube_publish as app

CHANNEL = 'UC' + 'a' * 22
OTHER = 'UC' + 'b' * 22
VIDEO = 'a' * 11
META = {'title': 'Synthetic demo', 'description': 'Public example', 'categoryId': '22',
        'tags': ['demo'], 'madeForKids': False}


def item(privacy='private', processing='succeeded', upload='processed', channel=CHANNEL):
    return {'id': VIDEO, 'snippet': {'channelId': channel},
            'status': {'privacyStatus': privacy, 'uploadStatus': upload},
            'processingDetails': {'processingStatus': processing}}


def api():
    result = Mock()
    result.channels.return_value.list.return_value.execute.return_value = {'items': [{'id': CHANNEL}]}
    result.videos.return_value.list.return_value.execute.return_value = {'items': [item()]}
    result.videos.return_value.insert.return_value.next_chunk.return_value = (None, {'id': VIDEO})
    return result


class CLITests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='social-publish-test-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.state = self.root / 'state'
        self.meta = self.root / 'metadata.json'
        self.meta.write_text(json.dumps(META))
        self.media = self.root / 'demo.mp4'
        self.media.write_bytes(b'synthetic video bytes')
        self.api = api()
        self.factory = Mock(return_value=self.api)
        self.media_factory = Mock(return_value=object())

    def args(self, command, *extra):
        options = [command, '--expected-channel-id', CHANNEL, '--state-dir', str(self.state), '--allow-network']
        if command == 'upload':
            options += ['--file', str(self.media), '--metadata', str(self.meta), '--allow-upload']
        if command in ('status', 'publish'):
            options += ['--video-id', VIDEO]
        if command == 'publish':
            options += ['--allow-publish', '--confirm-video-id', VIDEO]
        if command == 'auth':
            options += ['--client-config', str(self.root / 'client.json'), '--allow-auth']
        return app.parser().parse_args(options + list(extra))

    def run_cli(self, args):
        return app.run(args, self.factory, self.media_factory)

    def rejected(self, args, code):
        with self.assertRaises(app.Rejected) as caught:
            self.run_cli(args)
        self.assertEqual(caught.exception.code, code)

    def receipt(self):
        state = app.State(self.state)
        state.write(state.receipt_name(CHANNEL, 'synthetic-hash'),
                    {'channel_id': CHANNEL, 'video_id': VIDEO, 'private_readback': True})

    def test_channel(self):
        self.assertEqual(self.run_cli(self.args('channel')), {'channel_id': CHANNEL})
        self.api.channels.return_value.list.assert_called_once_with(part='id', mine=True)

    def test_approval_gates_before_state_or_network(self):
        for command, attribute, error in [('channel', 'allow_network', 'network_approval_required'),
                                         ('auth', 'allow_auth', 'auth_approval_required'),
                                         ('upload', 'allow_upload', 'upload_approval_required'),
                                         ('publish', 'allow_publish', 'publish_approval_required')]:
            with self.subTest(command=command):
                args = self.args(command)
                setattr(args, attribute, False)
                self.rejected(args, error)
        self.factory.assert_not_called()
        self.assertFalse(self.state.exists())

    def test_exact_confirmation(self):
        args = self.args('publish')
        args.confirm_video_id = 'b' * 11
        self.rejected(args, 'video_confirmation_mismatch')
        self.factory.assert_not_called()

    def test_identity_negatives_before_write(self):
        for ids in ([], [OTHER], [CHANNEL, OTHER], [CHANNEL, CHANNEL]):
            with self.subTest(ids=ids):
                self.api.channels.return_value.list.return_value.execute.return_value = {'items': [{'id': x} for x in ids]}
                self.rejected(self.args('upload'), 'channel_identity_mismatch')
                self.rejected(self.args('publish'), 'channel_identity_mismatch')
        self.api.videos.return_value.insert.assert_not_called()
        self.api.videos.return_value.update.assert_not_called()

    def test_upload_positive_private_receipt_and_duplicate(self):
        result = self.run_cli(self.args('upload'))
        self.assertEqual(result['video_id'], VIDEO)
        self.assertEqual(result['privacy'], 'private')
        body = self.api.videos.return_value.insert.call_args.kwargs['body']
        self.assertEqual(body['status']['privacyStatus'], 'private')
        state = app.State(self.state)
        self.assertTrue(state.uploaded(CHANNEL, VIDEO))
        self.rejected(self.args('upload'), 'duplicate_upload_or_ambiguous_attempt')
        self.assertEqual(self.api.videos.return_value.insert.call_count, 1)
        self.api.videos.return_value.insert.return_value.next_chunk.assert_called_once_with(num_retries=0)

    def test_ambiguous_failure_blocks_retry(self):
        self.api.videos.return_value.insert.return_value.next_chunk.side_effect = RuntimeError('PRIVATE_UPLOAD_URL')
        with self.assertRaises(RuntimeError):
            self.run_cli(self.args('upload'))
        self.rejected(self.args('upload'), 'duplicate_upload_or_ambiguous_attempt')
        self.assertFalse(app.State(self.state).uploaded(CHANNEL, VIDEO))

    def test_upload_readback_foreign_or_nonprivate(self):
        self.api.videos.return_value.list.return_value.execute.return_value = {'items': [item(channel=OTHER)]}
        self.rejected(self.args('upload'), 'video_channel_mismatch')
        self.assertFalse(app.State(self.state).uploaded(CHANNEL, VIDEO))

    def test_upload_nonprivate_readback(self):
        self.api.videos.return_value.list.return_value.execute.return_value = {'items': [item(privacy='public')]}
        self.rejected(self.args('upload'), 'upload_not_private')

    def test_publish_requires_receipt(self):
        self.rejected(self.args('publish'), 'private_upload_receipt_required')
        self.api.videos.return_value.update.assert_not_called()

    def test_publish_negative_states(self):
        self.receipt()
        cases = [(item(channel=OTHER), 'video_channel_mismatch'),
                 (item(privacy='public'), 'video_not_private'),
                 (item(privacy='unlisted'), 'video_not_private')]
        cases += [(item(processing=p), 'processing_not_succeeded')
                  for p in ('processing', 'failed', 'terminated', None)]
        cases += [(item(upload=p), 'processing_not_succeeded')
                  for p in ('uploaded', 'failed', 'rejected', 'deleted', None)]
        for value, code in cases:
            with self.subTest(value=value):
                self.api.videos.return_value.list.return_value.execute.return_value = {'items': [value]}
                self.rejected(self.args('publish'), code)
        self.api.videos.return_value.update.assert_not_called()

    def test_positive_upload_status_publish_flow(self):
        self.run_cli(self.args('upload'))
        self.assertEqual(self.run_cli(self.args('status'))['processing'], 'succeeded')
        self.api.videos.return_value.list.return_value.execute.side_effect = [
            {'items': [item()]}, {'items': [item(privacy='public')]}]
        self.assertEqual(self.run_cli(self.args('publish'))['privacy'], 'public')
        self.api.videos.return_value.update.assert_called_once_with(
            part='status', body={'id': VIDEO, 'status': {'privacyStatus': 'public'}})
        self.api.videos.return_value.update.return_value.execute.assert_called_once_with(num_retries=0)

    def test_status_rejects_foreign_or_missing_video(self):
        self.api.videos.return_value.list.return_value.execute.return_value = {'items': [item(channel=OTHER)]}
        self.rejected(self.args('status'), 'video_channel_mismatch')
        self.api.videos.return_value.list.return_value.execute.return_value = {'items': []}
        self.rejected(self.args('status'), 'video_not_found')

    def test_metadata_negatives(self):
        invalid = [None, {}, dict(META, privacyStatus='public'), dict(META, title=''),
                   dict(META, title='x' * 101), dict(META, title='<bad>'),
                   dict(META, description='x' * 5001), dict(META, description='\x00'),
                   dict(META, tags='tag'), dict(META, tags=['']), dict(META, tags=['x' * 501]),
                   dict(META, categoryId=22), dict(META, categoryId='abc'),
                   dict(META, madeForKids='false')]
        for value in invalid:
            with self.subTest(value=value):
                with self.assertRaises(app.Rejected):
                    app.metadata(value)
        self.meta.write_text(json.dumps(dict(META, privacyStatus='public')))
        self.rejected(self.args('upload'), 'invalid_metadata_fields')
        self.factory.assert_not_called()

    def test_sanitized_provider_errors(self):
        for error in (RuntimeError('https://secret.invalid/?token=DO_NOT_ECHO'),
                      ValueError('credential=DO_NOT_ECHO'), KeyboardInterrupt()):
            output = io.StringIO()
            with patch.object(app, 'run', side_effect=error), contextlib.redirect_stdout(output):
                self.assertEqual(app.main(['channel', '--expected-channel-id', CHANNEL]), 1)
            value = output.getvalue()
            self.assertNotIn('DO_NOT_ECHO', value)
            self.assertEqual(json.loads(value)['error'], 'operation_failed')

    def test_invalid_arguments_do_not_echo_values(self):
        output, errors = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(output), contextlib.redirect_stderr(errors):
            self.assertEqual(app.main(['channel', '--unexpected=DO_NOT_ECHO']), 1)
        self.assertNotIn('DO_NOT_ECHO', output.getvalue() + errors.getvalue())
        self.assertEqual(json.loads(output.getvalue())['error'], 'invalid_arguments')

    def test_provider_prints_and_logs_suppressed(self):
        def noisy(args):
            print('DO_NOT_ECHO')
            print('DO_NOT_ECHO', file=app.sys.stderr)
            app.logging.error('DO_NOT_ECHO')
            raise RuntimeError('DO_NOT_ECHO')
        output, errors = io.StringIO(), io.StringIO()
        with patch.object(app, 'run', side_effect=noisy), \
                contextlib.redirect_stdout(output), contextlib.redirect_stderr(errors):
            self.assertEqual(app.main(['channel', '--expected-channel-id', CHANNEL]), 1)
        self.assertNotIn('DO_NOT_ECHO', output.getvalue() + errors.getvalue())

    def test_identity_pagination_rejected(self):
        self.api.channels.return_value.list.return_value.execute.return_value = {
            'items': [{'id': CHANNEL}], 'nextPageToken': 'synthetic'}
        self.rejected(self.args('upload'), 'channel_identity_mismatch')
        self.api.videos.return_value.insert.assert_not_called()

    def test_media_and_metadata_symlinks_rejected(self):
        for original in (self.meta, self.media):
            linked = original.with_suffix('.linked')
            linked.symlink_to(original)
            args = self.args('upload')
            if original == self.meta:
                args.metadata = str(linked)
            else:
                args.file = str(linked)
            self.rejected(args, 'symlink_rejected')
        self.api.videos.return_value.insert.assert_not_called()


    def test_help_without_google_imports(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output), self.assertRaises(SystemExit) as exit_info:
            app.main(['--help'])
        self.assertEqual(exit_info.exception.code, 0)
        self.assertIn('upload', output.getvalue())

    def test_summary_does_not_echo_provider_fields(self):
        value = item(privacy='SECRET', processing='SECRET', upload='SECRET')
        self.assertNotIn('SECRET', json.dumps(app.summary(value)))


class AuthenticationTests(unittest.TestCase):
    setUp = CLITests.setUp
    args = CLITests.args
    run_cli = CLITests.run_cli
    rejected = CLITests.rejected
    # Keep these tests independent of installed Google packages.
    def test_refresh_is_persisted(self):
        state = app.State(self.state)
        state.write('token.json', {'scopes': app.SCOPES['upload']})
        credentials = Mock(expired=True, refresh_token=True, valid=True)
        credentials.to_json.return_value = json.dumps({'scopes': app.SCOPES['upload'], 'synthetic': 'refreshed'})
        constructor = Mock()
        constructor.from_authorized_user_info.return_value = credentials
        modules = {'google.oauth2.credentials': Mock(Credentials=constructor),
                   'google.auth.transport.requests': Mock(Request=Mock())}
        with patch.dict('sys.modules', modules), patch.object(app, 'build_api', return_value=self.api):
            self.assertIs(app.load_api(state, 'upload'), self.api)
        credentials.refresh.assert_called_once()
        self.assertEqual(state.read('token.json')['synthetic'], 'refreshed')

    def test_insufficient_scope_before_refresh(self):
        state = app.State(self.state)
        state.write('token.json', {'scopes': app.SCOPES['readonly']})
        constructor = Mock()
        with patch.dict('sys.modules', {'google.oauth2.credentials': Mock(Credentials=constructor),
                                       'google.auth.transport.requests': Mock()}):
            with self.assertRaises(app.Rejected):
                app.load_api(state, 'publish')
        constructor.from_authorized_user_info.assert_not_called()

    def test_transport_no_401_replay_and_late_refresh_persistence(self):
        state = app.State(self.state)
        credentials = Mock()
        credentials.to_json.return_value = json.dumps({'synthetic': 'late refresh'})
        original_refresh = credentials.refresh
        transport = Mock()
        build = Mock(return_value=self.api)
        modules = {'googleapiclient.discovery': Mock(build=build),
                   'google_auth_httplib2': Mock(AuthorizedHttp=transport),
                   'httplib2': Mock()}
        with patch.dict('sys.modules', modules):
            self.assertIs(app.build_api(credentials, state), self.api)
            credentials.refresh(object())
        self.assertEqual(transport.call_args.kwargs['max_refresh_attempts'], 0)
        self.assertNotIn('credentials', build.call_args.kwargs)
        original_refresh.assert_called_once()
        self.assertEqual(state.read('token.json')['synthetic'], 'late refresh')

    def client(self):
        path = self.root / 'client.json'
        path.write_text(json.dumps({'installed': {
            'auth_uri': 'https://accounts.google.com/o/oauth2/auth',
            'token_uri': 'https://oauth2.googleapis.com/token'}}))
        path.chmod(0o600)
        return path

    def test_auth_loopback_selection_then_identity_before_persistence(self):
        self.client()
        flow = Mock()
        credentials = flow.run_local_server.return_value
        credentials.to_json.return_value = json.dumps({'synthetic': True})
        installed_flow = Mock()
        installed_flow.from_client_config.return_value = flow
        with patch.dict('sys.modules', {'google_auth_oauthlib.flow': Mock(InstalledAppFlow=installed_flow)}), \
                patch.object(app, 'build_api', return_value=self.api):
            result = self.run_cli(self.args('auth'))
            self.assertTrue(result['authenticated'])
            options = flow.run_local_server.call_args.kwargs
            self.assertEqual(options['host'], '127.0.0.1')
            self.assertEqual(options['port'], 0)
            self.assertIn('select_account', options['prompt'])
            token = self.state / 'token.json'
            self.assertEqual(token.stat().st_mode & 0o777, 0o600)
            token.unlink()
            self.api.channels.return_value.list.return_value.execute.return_value = {'items': [{'id': OTHER}]}
            self.rejected(self.args('auth'), 'channel_identity_mismatch')
            self.assertFalse(token.exists())

    def test_client_config_insecure_or_non_google_endpoints(self):
        path = self.client()
        path.chmod(0o644)
        self.rejected(self.args('auth'), 'insecure_permissions')
        path.chmod(0o600)
        path.write_text(json.dumps({'installed': {'auth_uri': 'https://example.invalid',
                                                'token_uri': 'https://example.invalid'}}))
        self.rejected(self.args('auth'), 'invalid_oauth_endpoints')



class StorageTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='social-storage-test-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.state = app.State(self.root / 'state')

    def test_atomic_protected_tokens_and_receipts(self):
        for name in ('token.json', 'receipt-demo.json'):
            self.state.write(name, {'synthetic': True})
            self.assertEqual(self.state.read(name), {'synthetic': True})
            self.assertEqual(self.state.file(name).stat().st_mode & 0o777, 0o600)
            self.state.write(name, {'synthetic': False})
        self.assertEqual(self.state.path.stat().st_mode & 0o777, 0o700)
        self.assertEqual(list(self.state.path.glob('.atomic-*')), [])

    def test_hardlinked_token_rejected(self):
        target = self.root / 'synthetic.json'
        target.write_text('{}')
        target.chmod(0o600)
        os.link(target, self.state.path / 'token.json')
        with self.assertRaises(app.Rejected):
            self.state.read('token.json')


    def test_insecure_directory(self):
        self.state.path.chmod(0o755)
        with self.assertRaises(app.Rejected):
            app.State(self.state.path)

    def test_insecure_token_and_receipt(self):
        for name in ('token.json', 'receipt-demo.json'):
            path = self.state.path / name
            path.write_text('{}')
            path.chmod(0o644)
            with self.assertRaises(app.Rejected):
                self.state.read(name)
            with self.assertRaises(app.Rejected):
                self.state.write(name, {})

    def test_symlink_token_receipt_directory_and_ancestor(self):
        target = self.root / 'target'
        target.write_text('{}')
        target.chmod(0o600)
        for name in ('token.json', 'receipt-demo.json'):
            (self.state.path / name).symlink_to(target)
            with self.assertRaises(app.Rejected):
                self.state.read(name)
            with self.assertRaises(app.Rejected):
                self.state.write(name, {})
        link = self.root / 'alias'
        link.symlink_to(self.state.path, target_is_directory=True)
        for path in (link, link / 'child'):
            with self.assertRaises(app.Rejected):
                app.State(path)

    def test_dangling_symlink(self):
        (self.state.path / 'token.json').symlink_to(self.root / 'missing')
        with self.assertRaises(app.Rejected):
            self.state.write('token.json', {})

    def test_git_tree_refused(self):
        (self.root / '.git').mkdir()
        with self.assertRaises(app.Rejected):
            app.State(self.root / 'new-state')

    def test_lock_blocks_concurrency(self):
        with self.state.lock():
            with self.assertRaises(app.Rejected):
                with self.state.lock():
                    self.fail('second lock acquired')
        self.assertFalse((self.state.path / 'operation.lock').exists())

    def test_failed_atomic_replace_preserves_token(self):
        self.state.write('token.json', {'synthetic': 'old'})
        with patch.object(app.os, 'replace', side_effect=OSError('synthetic')):
            with self.assertRaises(OSError):
                self.state.write('token.json', {'synthetic': 'new'})
        self.assertEqual(self.state.read('token.json'), {'synthetic': 'old'})
        self.assertEqual(list(self.state.path.glob('.atomic-*')), [])


if __name__ == '__main__':
    unittest.main()
