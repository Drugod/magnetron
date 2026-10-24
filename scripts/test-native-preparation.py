"""Offline security/lock tests for native preparation, runnable on Windows too."""
import importlib.util,io,json,pathlib,tarfile,tempfile,unittest,zipfile
ROOT=pathlib.Path(__file__).resolve().parent.parent
spec=importlib.util.spec_from_file_location('prepare_native',ROOT/'scripts/prepare-native.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class NativePreparation(unittest.TestCase):
    def test_lock(self):m.check_lock()
    def test_tar_traversal(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp);archive=root/'evil.tar.gz'
            with tarfile.open(archive,'w:gz') as out:
                entry=tarfile.TarInfo('../outside.txt');entry.size=4;out.addfile(entry,io.BytesIO(b'evil'))
            with self.assertRaises((tarfile.TarError,ValueError)):m.untar(archive,root/'dest')
            self.assertFalse((root/'outside.txt').exists())
    def test_zip_traversal_and_symlink(self):
        for name,mode in [('../outside.txt',0),('/outside.txt',0),('symlink',0o120777<<16)]:
            with tempfile.TemporaryDirectory() as tmp:
                root=pathlib.Path(tmp);archive=root/'evil.zip'
                with zipfile.ZipFile(archive,'w') as out:
                    entry=zipfile.ZipInfo(name);entry.external_attr=mode;out.writestr(entry,'evil')
                with self.assertRaises(ValueError):m.unzip(archive,root/'dest')
    def test_safe_archives(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp);archive=root/'safe.zip'
            with zipfile.ZipFile(archive,'w') as out:out.writestr('folder/file.txt','ok')
            m.unzip(archive,root/'dest');self.assertEqual((root/'dest/folder/file.txt').read_text(),'ok')
if __name__=='__main__':unittest.main()
