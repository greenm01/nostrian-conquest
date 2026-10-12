import sys
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import contextlib
import io
import json
import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch
from nc_admin.cli import main, parser, request_for
from nc_admin.client import AdminError, Client

class AdminTests(unittest.TestCase):
    def test_preview_never_launches_process(self):
        output = io.StringIO()
        with patch('subprocess.run') as run, contextlib.redirect_stdout(output):
            self.assertEqual(main(['games','create','alpha','--players','4','--dry-run','--json']),0)
        run.assert_not_called()
        self.assertEqual(json.loads(output.getvalue())['data']['request']['action'],'create')

    def test_confirmation_and_timezone(self):
        args=parser().parse_args(['games','archive','alpha','--confirm','beta'])
        with self.assertRaises(AdminError):request_for(args)
        with contextlib.redirect_stderr(io.StringIO()),self.assertRaises(SystemExit):
            parser().parse_args(['games','start','alpha','--first-due','2026-10-20T04:00:00'])

    def test_arguments_are_passed_without_shell(self):
        result=subprocess.CompletedProcess([],0,json.dumps({'version':1,'ok':True,'data':[]}), '')
        with patch('subprocess.run',return_value=result) as run:
            self.assertEqual(Client(Path('/games with spaces'),'nc-host').call({'action':'list'}),[])
        self.assertEqual(run.call_args.args[0],['nc-host','admin','--root','/games with spaces'])
        self.assertNotIn('shell',run.call_args.kwargs)

    def test_rejects_failure_bad_json_and_version(self):
        for code,stdout in [(1,'{}'),(0,'not json'),(0,'{"version":2}'),(0,'[]')]:
            with patch('subprocess.run',return_value=subprocess.CompletedProcess([],code,stdout,'')):
                with self.assertRaises(AdminError):Client(Path('/games'),'nc-host').call({'action':'list'})

    def test_scheduler_and_backup_errors_have_nonzero_exit(self):
        for argv in (["maintenance","due","--json"],["backup","daily","--json"]):
            with patch("nc_admin.cli.Client.call",return_value=[{"game":"one","error":"disk full"}]),contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(main(argv),1)

    def test_private_invite_and_calendar_requests(self):
        cases = [
            (["games","invite-only","beta-1"], {"action":"invite_only","game":"beta-1"}),
            (["invites","create","beta-1","--count","3"], {"action":"create_invites","game":"beta-1","count":3}),
            (["invites","list","beta-1"], {"action":"list_invites","game":"beta-1"}),
            (["invites","revoke","beta-1","example-id"], {"action":"revoke_invite","game":"beta-1","invite_id":"example-id"}),
            (["games","schedule","beta-1","--midnight","America/New_York"], {"action":"configure_schedule","game":"beta-1","time_zone":"America/New_York"}),
            (["games","schedule","practice","--manual"], {"action":"configure_schedule","game":"practice","time_zone":None}),
            (["games","start","beta-1"], {"action":"start_configured","game":"beta-1"}),
            (["games","resume","beta-1"], {"action":"start_configured","game":"beta-1"}),
        ]
        for argv,expected in cases:
            with self.subTest(argv=argv): self.assertEqual(request_for(parser().parse_args(argv)),expected)
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            parser().parse_args(["games","schedule","beta-1","--manual","--midnight","America/New_York"])

    def test_timeout_never_retries_mutation(self):
        with patch('subprocess.run',side_effect=subprocess.TimeoutExpired('nc-host',1)) as run:
            with self.assertRaises(AdminError):Client(Path('/games'),'nc-host').call({'action':'maintain'})
        self.assertEqual(run.call_count,1)

if __name__=='__main__': unittest.main()
