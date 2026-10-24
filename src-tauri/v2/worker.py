"""Private stdio worker. Never runs a web server or installs a Python runtime."""
import json, os, pathlib, queue, re, sys, threading, time
import libtorrent as lt
import ipaddress, urllib.parse

def emit(**event):
    print(json.dumps(event, ensure_ascii=True), flush=True)

def paths_for(ti, root, resume):
    paths=[]; seen=set()
    for i in range(ti.files().num_files()):
        if ti.files().file_flags(i) & lt.file_storage.flag_pad_file: continue
        raw=ti.files().file_path(i).replace('\\','/')
        path=pathlib.PurePosixPath(raw)
        if path.is_absolute() or not path.parts: raise ValueError('Unsafe torrent path')
        current=root
        for part in path.parts:
            stem=part.split('.')[0].upper()
            if part in ('.','..') or part.endswith((' ','.')) or re.search(r'[<>:"|?*\x00-\x1f]',part) or stem in ('CON','PRN','AUX','NUL') or re.fullmatch(r'(COM|LPT)[1-9]',stem): raise ValueError('Unsafe torrent filename')
            current=current/part
            if current.exists() and (current.is_symlink() or getattr(current.lstat(),'st_file_attributes',0)&0x400): raise ValueError('Links and junctions are not allowed')
        if str(path).lower() in seen: raise ValueError('Duplicate Windows filename')
        seen.add(str(path).lower())
        if current.exists() and not resume: raise ValueError('A destination file already exists; it will not be overwritten')
        paths.append(str(path))
    if len(paths)>100000: raise ValueError('Too many torrent files')
    return paths

def run(config):
    folder=pathlib.Path(config['folder']).resolve(strict=True)
    staging=pathlib.Path(config['staging']).resolve(strict=True)
    if not folder.is_dir(): raise ValueError('Destination is not a folder')
    commands=queue.Queue()
    def read():
        for line in sys.stdin:
            if len(line)>1000000: break
            try: commands.put(json.loads(line))
            except Exception: break
        commands.put({'op':'stop'})
    threading.Thread(target=read,daemon=True).start()
    settings={'listen_interfaces':'0.0.0.0:0','enable_upnp':False,'enable_natpmp':False,'enable_lsd':False,'enable_dht':True,'max_metadata_size':4*1024*1024,'connections_limit':80,'active_downloads':1,'active_seeds':0,'close_redundant_connections':False,'alert_mask':int(lt.alert.category_t.error_notification|lt.alert.category_t.storage_notification|lt.alert.category_t.status_notification)}
    session=lt.session(settings)
    if config.get('verify') and not config.get('test_loopback'):
        rules=lt.ip_filter()
        for network in ['0.0.0.0/8','10.0.0.0/8','100.64.0.0/10','127.0.0.0/8','169.254.0.0/16','172.16.0.0/12','192.0.0.0/24','192.0.2.0/24','192.168.0.0/16','198.18.0.0/15','198.51.100.0/24','203.0.113.0/24','224.0.0.0/3','::/0']:
            subnet=ipaddress.ip_network(network);rules.add_rule(str(subnet.network_address),str(subnet.broadcast_address),1)
        session.set_ip_filter(rules)
    def limits(down,up): session.apply_settings({'download_rate_limit':int(down)*1024,'upload_rate_limit':int(up)*1024})
    limits(config.get('down',0),config.get('up',0))
    params=lt.parse_magnet_uri(config['uri'])
    peers=list(params.peers)
    for key,value in urllib.parse.parse_qsl(urllib.parse.urlsplit(config['uri']).query):
        if key=='x.pe':
            endpoint=urllib.parse.urlsplit('//'+value)
            ip=ipaddress.ip_address(endpoint.hostname)
            if not endpoint.port: raise ValueError('Invalid peer port')
            peer=(str(ip),endpoint.port)
            if peer not in peers:peers.append(peer)
    params.peers=peers
    expected=str(params.info_hashes.v2)
    if not params.info_hashes.has_v2() or params.info_hashes.has_v1(): raise ValueError('This worker handles pure BitTorrent v2 links')
    params.save_path=str(staging)
    params.flags &= ~lt.torrent_flags.auto_managed
    params.flags &= ~lt.torrent_flags.paused
    params.flags |= lt.torrent_flags.upload_mode
    handle=session.add_torrent(params)
    validated=False; moving=False; ready=False; finished=False; start=time.monotonic();last=0
    try:
        while True:
            while not commands.empty():
                command=commands.get_nowait()
                if command.get('op')=='stop': return
                if command.get('op')=='limits': limits(command.get('down',0),command.get('up',0))
            for alert in session.pop_alerts():
                if isinstance(alert,lt.storage_moved_alert):
                    moving=False;ready=True
                    if config.get('resume'): handle.force_recheck()
                    handle.unset_flags(lt.torrent_flags.upload_mode)
                if isinstance(alert,(lt.torrent_error_alert,lt.file_error_alert,lt.storage_moved_failed_alert)): raise RuntimeError(alert.message())
            status=handle.status()
            if status.has_metadata and not validated:
                ti=handle.torrent_file()
                if str(ti.info_hashes().v2)!=expected: raise ValueError('BitTorrent v2 metadata hash mismatch')
                paths=paths_for(ti,folder,bool(config.get('resume')))
                emit(kind='metadata',files=paths,total=int(ti.total_size()),name=ti.name())
                validated=True
                if config.get('verify'):
                    # No payload requests are made in upload mode. Admission still requires a full seed.
                    pass
                else:
                    handle.move_storage(str(folder),lt.move_flags_t.dont_replace if config.get('resume') else lt.move_flags_t.fail_if_exist);moving=True
            if config.get('verify') and validated:
                for peer in handle.get_peer_info():
                    if peer.flags & lt.peer_info.seed:
                        emit(kind='verified');return
            if ready and status.is_seeding:
                finished=True;return
            if time.monotonic()-last>=1:
                emit(kind='progress',completed=int(status.total_wanted_done),total=int(status.total_wanted),down=int(status.download_payload_rate),up=int(status.upload_payload_rate),state='checking' if status.state in (lt.torrent_status.checking_files,lt.torrent_status.checking_resume_data) else ('downloading' if ready else 'resolving'))
                last=time.monotonic()
            if not ready and time.monotonic()-start>(30 if config.get('verify') else 120): raise TimeoutError('No reachable complete v2 source in time' if config.get('verify') else 'BitTorrent v2 metadata or storage timed out')
            time.sleep(.1)
    finally:
        session.pause();session.remove_torrent(handle)
        del handle
        del session
        if finished: emit(kind='completed')

if __name__=='__main__':
    try:
        first=sys.stdin.readline(1000001)
        if len(first)>1000000: raise ValueError('Request too large')
        run(json.loads(first))
    except Exception as error:
        emit(kind='error',error=str(error));sys.exit(1)
