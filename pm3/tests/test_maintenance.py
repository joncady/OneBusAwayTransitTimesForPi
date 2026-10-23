"""Synthetic failure tests. No device, personal config or real disk writes."""
import gzip
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch
from contextlib import nullcontext

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
import retention
from retention import select_expired, release_refs, USB
spec = importlib.util.spec_from_file_location('recovery', ROOT / 'verify-recovery.py')
recovery = importlib.util.module_from_spec(spec)
spec.loader.exec_module(recovery)


class RetentionTests(unittest.TestCase):
    def test_keep_newest_and_every_referenced_version(self):
        self.assertEqual(select_expired({'a': 1, 'b': 2, 'c': 3, 'd': 4}, 2, {'a'}), ['b'])
        with self.assertRaises(ValueError):select_expired({}, 0)

    def test_paths_are_parsed_not_substring_matched(self):
        token = 'a'*32
        self.assertEqual(release_refs({'albums': {'Test': f'{USB}/{token}/album'}}), {token})
        self.assertEqual(release_refs({'albums': {'Test': f'/other/{token}/album'}}), set())
        with self.assertRaises(ValueError):release_refs({'albums': {'Test': f'{USB}/{token}/../other'}})
        with self.assertRaises(ValueError):release_refs({'albums': []})

    def test_cleanup_protects_active_previous_and_retained_snapshot_media(self):
        media = [f'{n:032x}' for n in range(1, 11)]
        snapshots = [f'20261022T000000Z-{n}' for n in range(1, 5)]
        inventory = ''.join(f'media|{name}|{i+1 if i<9 else 0}\n' for i,name in enumerate(media))
        inventory += ''.join(f'deployment|{name}|{i+1}\n' for i,name in enumerate(snapshots))
        files = {'actions.local.json': media[0], 'actions.local.json.previous': media[1]}
        files.update({f'deployments/{name}/actions.local.json': media[i+2] for i,name in enumerate(snapshots)})
        stream=io.BytesIO()
        with tarfile.open(fileobj=stream, mode='w') as archive:
            for name,release in files.items():
                data=json.dumps({'albums': {'Test': f'{USB}/{release}/album'}}).encode()
                entry=tarfile.TarInfo(name);entry.size=len(data);archive.addfile(entry,io.BytesIO(data))
        applied=[]
        def fake_ssh(target, command, **kwargs):
            if command==retention.INVENTORY:data=inventory.encode()
            elif 'cat ' in command and 'last-good' in command:data=f'deployments/{snapshots[0]}'.encode()
            elif 'tar -cf' in command:data=stream.getvalue()
            else:applied.append(command);data=b''
            return subprocess.CompletedProcess([],0,stdout=data)
        with patch.object(retention,'ssh',fake_ssh),patch.object(retention,'maintenance_lock',lambda _:nullcontext()):
            old_media,old_snapshots=retention.cleanup('unused',apply=True)
        self.assertEqual(old_media,[media[9]])
        self.assertEqual(old_snapshots,[])
        self.assertIn(f'rm -rf -- {USB}/{media[9]}',applied[0])
        for name in media[:9]:self.assertNotIn(f'rm -rf -- {USB}/{name}',applied[0])

    def test_invalid_last_good_pointer_refuses_cleanup(self):
        def fake_ssh(target,command,**kwargs):
            data=b'' if command==retention.INVENTORY else b'deployments/../../unrelated'
            return subprocess.CompletedProcess([],0,stdout=data)
        with patch.object(retention,'ssh',fake_ssh),patch.object(retention,'maintenance_lock',lambda _:nullcontext()):
            with self.assertRaises(ValueError):retention.cleanup('unused',apply=True)


class RecoveryTests(unittest.TestCase):
    def make_backup(self, root):
        sizes = {'root-card.img.gz': 1024, 'data-card.img.gz': 2048, 'spi.img.gz': 512}
        for name, size in sizes.items():
            with gzip.open(root / name, 'wb') as f:f.write(bytes(size))
        with tarfile.open(root / 'usb-files.tar.gz', 'w:gz') as archive:
            entry = tarfile.TarInfo('autorun.sh');entry.size = 4
            archive.addfile(entry, io.BytesIO(b'test'))
        (root / 'images.sha256').write_text(''.join(hashlib.sha256((root/n).read_bytes()).hexdigest()+'  '+n+'\n' for n in [*sizes, 'usb-files.tar.gz']))
        return sizes

    def test_restore_to_new_regular_files_and_refuse_overwrite(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp);sizes = self.make_backup(root);out = root / 'rehearsal'
            report = recovery.verify(root, out, sizes)
            self.assertEqual(report['data-card.img.gz']['rawBytes'], 2048)
            self.assertEqual((out / 'data-card.img').read_bytes(), bytes(2048))
            with self.assertRaises(FileExistsError):recovery.verify(root, out, sizes)

    def test_corrupt_compressed_input_is_rejected_before_restore(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp);sizes = self.make_backup(root)
            (root/'spi.img.gz').write_bytes(b'corrupt')
            with self.assertRaises(ValueError):recovery.verify(root, root/'restore', sizes)
            self.assertFalse((root/'restore').exists())


class ActivationTests(unittest.TestCase):
    def run_case(self, failure=None):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp);home = root/'app';home.mkdir();(home/'maintenance.lock').mkdir()
            incoming=home/'update-incoming';incoming.mkdir()
            tools=root/'bin';tools.mkdir()
            def executable(path, text):path.write_text(text);path.chmod(0o700)
            good='#!/bin/sh\nexit 0\n'
            bad='#!/bin/sh\n[ "$1" = --check-config ] && exit 0\nexit 1\n'
            restart=f'#!/bin/sh\n"{home}/prototype" --run || exit 1\necho active > "{home}/state"\n'
            for folder in [home,incoming]:
                executable(folder/'prototype', good)
                executable(folder/'restart-display.sh', restart)
                executable(folder/'start-display.sh', '#!/bin/sh\nexit 0\n')
            candidate = good + '# candidate build\n'
            executable(incoming/'prototype', candidate)
            (home/'actions.local.json').write_text('{"albums":{"Private":"unchanged"}}')
            (home/'state').write_text('active')
            executable(tools/'systemctl', f'''#!/bin/sh
case "$1" in
show) echo loaded;;
stop) echo inactive > "{home}/state";;
is-active) [ "$(cat "{home}/state")" = active ];;
*) exit 0;;
esac
''')
            if failure=='startup':executable(incoming/'prototype',bad)
            if failure=='transfer':(incoming/'start-display.sh').unlink()
            if failure=='swap':
                executable(tools/'mv', f'''#!/bin/sh
if [ "$1" = update-incoming/restart-display.sh ]; then exit 1; fi
exec /bin/mv "$@"
''')
            if failure=='signal':
                executable(tools/'mv', '#!/bin/sh\nif [ "$1" = update-incoming/restart-display.sh ]; then kill -TERM "$PPID"; exit 1; fi\nexec /bin/mv "$@"\n')
            script=(ROOT/'activate-update.sh').read_text().replace('/init/transit-prototype',str(home))
            script=script.replace('sleep 2','sleep 0')
            result=subprocess.run(['sh','-s','--','60'],input=script,text=True,capture_output=True,env={**os.environ,'PATH':str(tools)+':'+os.environ['PATH']})
            self.assertEqual(result.returncode==0, failure is None, result.stderr)
            self.assertEqual((home/'state').read_text().strip(),'active')
            self.assertEqual((home/'prototype').read_text(),candidate if failure is None else good)
            self.assertEqual((home/'actions.local.json').read_text(),'{"albums":{"Private":"unchanged"}}')
            self.assertFalse((home/'cards.local.json').exists())

    def test_success(self):self.run_case()
    def test_failed_transfer_keeps_running_app(self):self.run_case('transfer')
    def test_startup_failure_restores_prior_app_and_config(self):self.run_case('startup')
    def test_partial_swap_restores_prior_app_and_config(self):self.run_case('swap')
    def test_termination_during_swap_restores_prior_app(self):self.run_case('signal')

class PublicationTests(unittest.TestCase):
    def run_case(self, failure=None):
        with tempfile.TemporaryDirectory() as tmp:
            home=Path(tmp);old='{"albums":{"Test":"old"}}';new='{"albums":{"Test":"new"}}'
            (home/'maintenance.lock').mkdir()
            (home/'actions.local.json').write_text(old)
            (home/'actions.local.json.new').write_text(new)
            def executable(path, text):path.write_text(text);path.chmod(0o700)
            executable(home/'prototype', '#!/bin/sh\nexit '+('1' if failure=='validation' else '0')+'\n')
            restart='#!/bin/sh\n'
            if failure=='startup':restart+='grep -q new actions.local.json && exit 1\n'
            restart+='exit 0\n'
            executable(home/'restart-display.sh',restart)
            tools=home/'bin';tools.mkdir();executable(tools/'systemctl','#!/bin/sh\nexit 0\n')
            script=(ROOT/'publish-actions.sh').read_text().replace('/init/transit-prototype',str(home)).replace('sleep 2','sleep 0')
            result=subprocess.run(['sh','-s'],input=script,text=True,capture_output=True,env={**os.environ,'PATH':str(tools)+':'+os.environ['PATH']})
            self.assertEqual(result.returncode==0,failure is None,result.stderr)
            self.assertEqual((home/'actions.local.json').read_text(),new if failure is None else old)
            if failure is None:self.assertEqual((home/'actions.local.json.previous').read_text(),old)

    def test_publish_success_keeps_previous_map(self):self.run_case()
    def test_invalid_map_keeps_current(self):self.run_case('validation')
    def test_failed_restart_restores_prior_map(self):self.run_case('startup')

if __name__=='__main__':unittest.main()
