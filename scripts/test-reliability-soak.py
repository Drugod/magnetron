"""Time-bounded repeated synthetic transfers, crashes, corrupt resume and persistence."""
import argparse,json,os,pathlib,subprocess,sys,time,datetime
ROOT=pathlib.Path(__file__).resolve().parents[1]
def main():
    parser=argparse.ArgumentParser();parser.add_argument('--minutes',type=float,default=30);args=parser.parse_args()
    if not 1<=args.minutes<=120:raise SystemExit('Soak duration must be 1..120 minutes')
    os.chdir(ROOT)
    built=subprocess.run(['cargo','test','--locked','--manifest-path','src-tauri/Cargo.toml','--no-run','--message-format=json'],capture_output=True,text=True,check=True)
    executables=[]
    for line in built.stdout.splitlines():
        try:message=json.loads(line)
        except json.JSONDecodeError:continue
        if message.get('reason')=='compiler-artifact' and message.get('profile',{}).get('test') and message.get('target',{}).get('name')=='magnet' and message.get('executable'):executables.append(message['executable'])
    if len(set(executables))!=1:raise SystemExit('Expected one compiled application test harness')
    executable=executables[0]
    scenarios=[
      'downloads::tests::metadata_download_pause_restart_resume_and_cancel_keep_files',
      'ed2k_downloads::tests::bundled_engine_add_pause_stop_resume_restart',
      'ed2k_downloads::tests::real_ed2k_transfer_and_completed_file_is_unlocked',
      'v2::tests::native_v2_pause_resume_and_delete',
      'engine::tests::bundled_engine_recovers_after_owned_process_crash',
      'secure_fs::tests::corrupted_queue_recovers_valid_checkpoint_and_preserves_damaged_data',
      'secure_fs::tests::corrupt_primary_and_backup_do_not_prevent_startup_or_delete_payloads',
    ]
    if os.name=='posix':scenarios+=['process_guard::tests::dropping_guard_terminates_the_owned_engine']
    report={'started_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'commit':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'platform':sys.platform,'requested_minutes':args.minutes,'status':'running','scenarios':[]}
    output=ROOT/'artifacts/reliability';output.mkdir(parents=True,exist_ok=True)
    started=time.monotonic();deadline=started+args.minutes*60;cycle=0
    try:
        while time.monotonic()<deadline:
            cycle+=1
            for name in scenarios:
                if time.monotonic()>=deadline:break
                before=time.monotonic()
                result=subprocess.run([executable,'--exact',name,'--test-threads=1'],capture_output=True,text=True,timeout=180)
                entry={'cycle':cycle,'name':name,'seconds':round(time.monotonic()-before,3),'success':result.returncode==0}
                report['scenarios'].append(entry)
                if result.returncode or '1 passed' not in result.stdout:
                    entry['diagnostic']=(result.stdout+'\n'+result.stderr)[-16000:]
                    raise RuntimeError('Reliability scenario failed: '+name)
            print('Completed recovery cycle',cycle,'elapsed seconds',int(time.monotonic()-started),flush=True)
        report['status']='passed'
    except BaseException:
        report['status']='failed';raise
    finally:
        report['elapsed_seconds']=round(time.monotonic()-started,3);report['cycles']=cycle
        (output/'report.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
    print('PASS:',len(report['scenarios']),'synthetic recovery scenarios over',report['elapsed_seconds'],'seconds')
if __name__=='__main__':main()
