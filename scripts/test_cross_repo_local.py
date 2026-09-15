from argparse import Namespace
from unittest import TestCase
from unittest.mock import patch

from scripts import cross_repo_local


class AutoOffForCommitTests(TestCase):
    def args(self) -> Namespace:
        return Namespace(no_fetch=True, staged=True, fix=True)

    def test_auto_off_and_stage_lock_when_sibling_is_clean_and_pushed(self) -> None:
        with (
            patch.object(cross_repo_local, "local_mode_is_on", return_value=True),
            patch.object(
                cross_repo_local,
                "sibling_revs",
                return_value=("a" * 40, "a" * 40, True),
            ),
            patch.object(cross_repo_local, "sibling_worktree_is_clean", return_value=True),
            patch.object(cross_repo_local, "disable_local_mode") as disable,
            patch.object(cross_repo_local, "run") as run,
            patch.object(cross_repo_local, "info"),
            patch.object(cross_repo_local, "report_status", return_value=0) as report,
        ):
            result = cross_repo_local.cmd_status(self.args())

        self.assertEqual(result, 0)
        disable.assert_called_once_with(keep_lock=False, refresh_sibling=False)
        run.assert_called_once_with(["git", "add", "--", "Cargo.lock"])
        report.assert_called_once_with(after_off=True, staged=True)

    def test_does_not_change_anything_when_sibling_head_is_not_pushed(self) -> None:
        with (
            patch.object(cross_repo_local, "local_mode_is_on", return_value=True),
            patch.object(
                cross_repo_local,
                "sibling_revs",
                return_value=("a" * 40, "b" * 40, False),
            ),
            patch.object(cross_repo_local, "sibling_worktree_is_clean", return_value=True),
            patch.object(cross_repo_local, "disable_local_mode") as disable,
            patch.object(cross_repo_local, "run") as run,
            patch.object(cross_repo_local, "report_status", return_value=1) as report,
        ):
            result = cross_repo_local.cmd_status(self.args())

        self.assertEqual(result, 1)
        disable.assert_not_called()
        run.assert_not_called()
        report.assert_called_once_with(after_off=False, staged=True, fix=True)

    def test_does_not_change_anything_when_sibling_is_missing(self) -> None:
        with (
            patch.object(cross_repo_local, "local_mode_is_on", return_value=True),
            patch.object(cross_repo_local, "sibling_revs", return_value=(None, None, False)),
            patch.object(cross_repo_local, "disable_local_mode") as disable,
            patch.object(cross_repo_local, "run") as run,
            patch.object(cross_repo_local, "report_status", return_value=1) as report,
        ):
            result = cross_repo_local.cmd_status(self.args())

        self.assertEqual(result, 1)
        disable.assert_not_called()
        run.assert_not_called()
        report.assert_called_once_with(after_off=False, staged=True, fix=True)

    def test_does_not_change_anything_when_sibling_worktree_is_dirty(self) -> None:
        with (
            patch.object(cross_repo_local, "local_mode_is_on", return_value=True),
            patch.object(
                cross_repo_local,
                "sibling_revs",
                return_value=("a" * 40, "a" * 40, True),
            ),
            patch.object(cross_repo_local, "sibling_worktree_is_clean", return_value=False),
            patch.object(cross_repo_local, "disable_local_mode") as disable,
            patch.object(cross_repo_local, "run") as run,
            patch.object(cross_repo_local, "report_status", return_value=1) as report,
        ):
            result = cross_repo_local.cmd_status(self.args())

        self.assertEqual(result, 1)
        disable.assert_not_called()
        run.assert_not_called()
        report.assert_called_once_with(after_off=False, staged=True, fix=True)

    def test_manual_off_uses_shared_disable_path(self) -> None:
        with (
            patch.object(cross_repo_local, "disable_local_mode") as disable,
            patch.object(cross_repo_local, "report_status", return_value=0) as report,
        ):
            result = cross_repo_local.cmd_off(Namespace(keep_lock=True))

        self.assertEqual(result, 0)
        disable.assert_called_once_with(keep_lock=True, refresh_sibling=True)
        report.assert_called_once_with(after_off=True)

    def test_off_mode_keeps_existing_stale_lock_fix_path(self) -> None:
        with (
            patch.object(cross_repo_local, "local_mode_is_on", return_value=False),
            patch.object(cross_repo_local, "disable_local_mode") as disable,
            patch.object(cross_repo_local, "run") as run,
            patch.object(cross_repo_local, "report_status", return_value=0) as report,
        ):
            result = cross_repo_local.cmd_status(self.args())

        self.assertEqual(result, 0)
        disable.assert_not_called()
        run.assert_not_called()
        report.assert_called_once_with(after_off=False, staged=True, fix=True)
