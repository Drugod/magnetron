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
subprocess.run([sys.executable,str(ROOT/'scripts/test-package-contents.py'),str(resources)],check=True)
runtime=resources/'v2';python=runtime/'bin/python3';worker=runtime/'worker.py'
if not python.is_file() or not worker.is_file():raise SystemExit('Package is missing its runtime or worker')
env=os.environ.copy();env['MAGNETRON_TEST_WORKER']=str(worker)
subprocess.run([str(python),'-I',str(ROOT/'scripts/test-v2.py')],env=env,check=True)
env['MAGNETRON_VIDEO_RUNTIME']=str(runtime)
subprocess.run([str(python),'-I',str(ROOT/'scripts/test-video.py')],env=env,check=True)
spec=importlib.util.spec_from_file_location('prepare_native',ROOT/'scripts/prepare-native.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);module.check_daemon_version(daemon)
if sys.platform=='linux':
    packages=list((ROOT/'src-tauri/target/release/bundle/deb').glob('*.deb'))
    if len(packages)!=1:raise SystemExit('Expected one Debian package')
    stage=ROOT/'vendor/native-deb-check'
    if stage.exists():shutil.rmtree(stage)
    stage.mkdir(parents=True)
    subprocess.run(['dpkg-deb','-x',str(packages[0]),str(stage)],check=True)
    candidates=list(stage.rglob('v2/worker.py'))
    if len(candidates)!=1:raise SystemExit('Debian package is missing its transfer worker')
    subprocess.run([sys.executable,str(ROOT/'scripts/test-package-contents.py'),str(candidates[0].parents[1])],check=True)
    dependencies=subprocess.check_output(['dpkg-deb','-f',str(packages[0]),'Depends'],text=True)
    if 'libwebkit2gtk-4.1' not in dependencies:raise SystemExit('Debian package must declare WebKit 4.1 dependency')
elif sys.platform=='darwin':
    import plistlib
    images=list((ROOT/'src-tauri/target/release/bundle/dmg').glob('*.dmg'))
    if len(images)!=1:raise SystemExit('Expected one macOS disk image')
    result=subprocess.check_output(['hdiutil','attach','-readonly','-nobrowse','-plist',str(images[0])])
    mounts=[pathlib.Path(e['mount-point']) for e in plistlib.loads(result)['system-entities'] if 'mount-point' in e]
    if len(mounts)!=1:raise SystemExit('Expected one disk-image mount')
    try:
        app=mounts[0]/'MAGNETRON.app'
        subprocess.run([sys.executable,str(ROOT/'scripts/test-package-contents.py'),str(app/'Contents/Resources')],check=True)
        installed=plistlib.loads((app/'Contents/Info.plist').read_bytes())
        built=plistlib.loads((resources.parent/'Info.plist').read_bytes())
        if installed['CFBundleShortVersionString']!=built['CFBundleShortVersionString']:raise SystemExit('Disk image application version differs')
    finally:subprocess.run(['hdiutil','detach',str(mounts[0])],check=True)
print('Packaged runtime, daemon, resource safety and distribution-container checks passed')
