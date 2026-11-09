"""Independent review regressions from videos.update's replacement contract."""
import unittest

import test_youtube_publish as base

VIDEO, item = base.VIDEO, base.item


class PublicationReviewTests(unittest.TestCase):
    setUp = base.CLITests.setUp
    args = base.CLITests.args
    run_cli = base.CLITests.run_cli
    rejected = base.CLITests.rejected
    receipt = base.CLITests.receipt

    def test_preserves_writable_status_fields(self):
        self.receipt()
        private = item()
        preserved = {'embeddable': False, 'license': 'creativeCommon',
                     'publicStatsViewable': False, 'selfDeclaredMadeForKids': False,
                     'containsSyntheticMedia': True}
        private['status'].update(preserved)
        private['status']['failureReason'] = 'not-a-writable-property'
        public = item(privacy='public')
        public['status'].update(preserved)
        self.api.videos.return_value.list.return_value.execute.side_effect = [
            {'items': [private]}, {'items': [public]}]
        self.assertEqual(self.run_cli(self.args('publish'))['privacy'], 'public')
        self.api.videos.return_value.update.assert_called_once_with(
            part='status', body={'id': VIDEO,
                                'status': dict(preserved, privacyStatus='public')})

    def test_scheduled_video_not_implicitly_unscheduled(self):
        self.receipt()
        scheduled = item()
        scheduled['status']['publishAt'] = '2099-01-01T00:00:00Z'
        self.api.videos.return_value.list.return_value.execute.return_value = {'items': [scheduled]}
        self.rejected(self.args('publish'), 'scheduled_video_requires_manual_review')
        self.api.videos.return_value.update.assert_not_called()


if __name__ == '__main__':
    unittest.main()
