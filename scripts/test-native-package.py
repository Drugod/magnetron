"""Exercise the runtime from the built package, after bundling/relocation."""
import importlib.util,os,pathlib,shutil,subprocess,sys
ROOT=pathlib.Path(__file__).resolve().parent.parent
if sys.platform=='darwin':
    resources=ROOT/'src-tauri/target/release/bundle/macos/MAGNETRON.app/Contents/Resources'
    daemon=resources/'amule/aMule.app/Contents/MacOS/amuled'
elif sys.platform=='linux':
    images=list((ROOT/'src-tauri/target/release/bundle/appimage').glob('*.AppImage'))
    if len(images)!=1:raise SystemExit('Expected one built AppImage')
    stage=ROOT/'vendor/native-package-check'
    if stage.exists():shutil.rmtree(stage)
    stage.mkdir(parents=True)
    env=os.environ.copy();env.pop('APPIMAGE_EXTRACT_AND_RUN',None)
    subprocess.run([str(images[0]),'--appimage-extract'],cwd=stage,env=env,check=True,stdout=subprocess.DEVNULL)
    candidates=list((stage/'squashfs-root').rglob('v2/bin/python3'))
    if len(candidates)!=1:raise SystemExit('Expected one packaged Python runtime')
    resources=candidates[0].parents[2]
    daemon=resources/'amule/bin/amuled'
else:raise SystemExit('Run on Linux or macOS')
runtime=resources/'v2';python=runtime/'bin/python3';worker=runtime/'worker.py'
if not python.is_file() or not worker.is_file():raise SystemExit('Package is missing its runtime or worker')
env=os.environ.copy();env['MAGNETRON_TEST_WORKER']=str(worker)
subprocess.run([str(python),'-I',str(ROOT/'scripts/test-v2.py')],env=env,check=True)
spec=importlib.util.spec_from_file_location('prepare_native',ROOT/'scripts/prepare-native.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);module.check_daemon_version(daemon)
print('Packaged runtime and daemon checks passed')
