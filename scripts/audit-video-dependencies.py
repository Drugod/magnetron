"""Audit pinned Python video wheels against OSV without installing packages."""
import json,pathlib,re,time,urllib.request
ROOT=pathlib.Path(__file__).resolve().parents[1]
lock=json.loads((ROOT/'scripts/video-assets.json').read_text(encoding='utf-8'))
queries=[]
for wheel in lock['wheels']:
    match=re.search(r'-([0-9][^-]*)-py3-none-any[.]whl$',wheel['url'])
    if not match:raise SystemExit('Unrecognised pinned wheel version')
    queries.append({'package':{'ecosystem':'PyPI','name':wheel['name'].replace('_','-')},'version':match[1]})
request=urllib.request.Request('https://api.osv.dev/v1/querybatch',data=json.dumps({'queries':queries}).encode(),headers={'Content-Type':'application/json','User-Agent':'MAGNETRON-security-check'},method='POST')
for attempt in range(3):
    try:
        with urllib.request.urlopen(request,timeout=30) as response:
            data=response.read(1024*1024+1)
            if len(data)>1024*1024:raise ValueError('Advisory response exceeds limit')
            result=json.loads(data)
        break
    except (OSError,ValueError):
        if attempt==2:raise
        time.sleep(attempt+1)
if len(result.get('results',[]))!=len(queries):raise SystemExit('Incomplete advisory response')
output=ROOT/'artifacts/security';output.mkdir(parents=True,exist_ok=True)
(output/'video-osv.json').write_text(json.dumps({'queries':queries,'response':result},indent=2)+'\n',encoding='utf-8')
failures=[]
for query,response in zip(queries,result['results']):
    ids=[v['id'] for v in response.get('vulns',[])]
    print(query['package']['name'],query['version'],':',', '.join(ids) if ids else 'no known OSV advisories')
    failures.extend(ids)
if failures:raise SystemExit('Known advisories in bundled video dependencies')
