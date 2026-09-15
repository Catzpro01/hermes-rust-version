"""Real PTY evidence for remaining Spec 017 UI; no UI text is reconstructed.

Readiness labels route keystrokes only; original streams and input events are
retained. Errors are explicit per case, never promoted to reviewed parity.
"""
import base64
import errno
import fcntl
import gzip
import hashlib
import json
import os
from pathlib import Path
import pty
import select
import signal
import sqlite3
import struct
import subprocess
import sys
import tempfile
import termios
import time

ROOT = Path(__file__).resolve().parents[1]
UPSTREAM = '63279301bcbdc185c1b07b98a9312eb0c862f26d'
SID_A = '550e8400-e29b-41d4-a716-446655440000'
SID_B = '660f8400-e29b-41d4-a716-446655440001'
# Requested explicitly by the size-contract capture; never part of the default
# matrix, so the existing bundles keep their exact case list.
PICKER_SIZE_CASES = ('picker-narrow', 'picker-too-small')
CASES = ('wizard-mode', 'wizard-full', 'wizard-blank', 'wizard-quick',
         'wizard-model', 'wizard-terminal', 'wizard-local', 'wizard-docker',
         'wizard-gateway', 'wizard-gateway-empty', 'wizard-gateway-token',
         'wizard-tools', 'wizard-tools-toggle', 'wizard-cancel',
         'picker-normal', 'picker-empty', 'picker-filter', 'picker-no-match',
         'picker-delete', 'completion-command', 'completion-subcommand',
         'completion-alternatives', 'summary-zero', 'summary-nonzero')


def record(command, home, width, steps, extra_env=None, timeout=15):
    """`steps` are (marker, key) pairs. A key of the form `@resize WxH` does
    not type anything: it re-sets the PTY winsize mid-session (the kernel then
    delivers SIGWINCH to the child), exercising live terminal resizes. Every
    other key is written to the PTY verbatim."""
    env = {'PATH': os.defpath, 'HOME': str(home), 'HERMES_HOME': str(home),
           'TERM': 'xterm-256color', 'COLORTERM': 'truecolor', 'LANG': 'C.UTF-8',
           'LC_ALL': 'C.UTF-8', 'PYTHONDONTWRITEBYTECODE': '1'}
    env.update(extra_env or {})
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 30, width, 0, 0))

    def session():
        os.setsid()
        fcntl.ioctl(slave, termios.TIOCSCTTY, 0)

    proc = subprocess.Popen(command, stdin=slave, stdout=slave, stderr=slave,
                            cwd=home, env=env, preexec_fn=session)
    os.close(slave)
    begin = last = stage_begin = time.monotonic()
    output = bytearray()
    events, inputs, completed = [], [], []
    index = cursor = 0
    stable_since = begin
    previous_screen = None
    error = None
    import pyte
    screen = pyte.Screen(width, 30)
    def terminal_reply(text):
        reply = text.encode()
        os.write(master, reply)
        inputs.append([round(time.monotonic()-begin, 6), base64.b64encode(reply).decode(), 'terminal response (pyte 0.8.2)'])
    screen.write_process_input = terminal_reply
    terminal = pyte.ByteStream(screen)
    eof = False
    cols_now, rows_now = width, 30
    try:
        while index < len(steps):
            now = time.monotonic()
            if now - stage_begin > timeout:
                # Diagnose without changing the snapshot rule: say what the child
                # actually drew, so a rare start-up failure is identifiable from
                # the annotation instead of only from the blocked job log.
                visible = [line.strip() for line in screen.display if line.strip()][:3]
                head = output[:160].decode('utf-8', 'replace')
                tail = output[-120:].decode('utf-8', 'replace')
                error = (f'missing readiness/timeout at stage {index}: {steps[index][0]} '
                         f'({len(output)} bytes; screen={visible!r}; '
                         f'head={head!r}; tail={tail!r})')
                break
            if not eof and select.select([master], [], [], 0.05)[0]:
                try:
                    data = os.read(master, 65536)
                except OSError as exc:
                    if exc.errno != errno.EIO:
                        raise
                    data = b''
                if data:
                    output.extend(data)
                    events.append([round(time.monotonic()-begin, 6), base64.b64encode(data).decode()])
                    last = time.monotonic()
                    if len(output) > 2_000_000:
                        error = 'output limit exceeded'
                        break
                    # Respond using terminal state, never hard-code cursor 1,1.
                    terminal.feed(data)
                    state = (screen.cursor.x, screen.cursor.y,
                             tuple(tuple(screen.buffer[y][x] for x in range(cols_now)) for y in range(rows_now)))
                    if state != previous_screen:
                        previous_screen = state
                        stable_since = time.monotonic()
                else:
                    eof = True
            elif eof:
                # poll() reaps an exited child; bounded wait avoids a hot loop.
                select.select([], [], [], 0.02)
            marker, key = steps[index]
            code = proc.poll()
            ready = (marker.encode() in bytes(output[cursor:]) or (len(output) > cursor and marker in '\n'.join(screen.display))) if marker != '@exit' else code is not None and eof
            if code is not None and not eof:
                continue  # Child exit does not mean its PTY output has been drained.
            if code is not None and code != 0:
                error = f'process exit {code} at stage {index}'
                break
            if ready and (time.monotonic()-last >= 0.3 or time.monotonic()-stable_since >= 0.3 or code is not None):
                completed.append({'marker': marker, 'end_byte': len(output)})
                index += 1
                if key is not None:
                    if key.startswith('@resize '):
                        new_cols, new_rows = (int(part) for part in key[len('@resize '):].split('x'))
                        cols_now, rows_now = new_cols, new_rows
                        fcntl.ioctl(master, termios.TIOCSWINSZ,
                                    struct.pack('HHHH', new_rows, new_cols, 0, 0))
                        screen.resize(new_rows, new_cols)
                        inputs.append([round(time.monotonic()-begin, 6),
                                       base64.b64encode(key.encode()).decode(),
                                       f'scenario resize {new_cols}x{new_rows}'])
                    else:
                        data = key.encode()
                        os.write(master, data)
                        inputs.append([round(time.monotonic()-begin, 6), base64.b64encode(data).decode(), 'scenario key'])
                    cursor = len(output)
                    stage_begin = last = stable_since = time.monotonic()
            elif code is not None:
                error = f'missing readiness at stage {index}: {marker}'
                break
        return {'command': command, 'environment': env, 'cwd': str(home),
                'width': width, 'height': 30, 'events_base64': events,
                'inputs_base64': inputs, 'raw_base64': base64.b64encode(output).decode(),
                'raw_sha256': hashlib.sha256(output).hexdigest(),
                'snapshot_end_byte': len(output), 'snapshot_rule': 'observed readiness + 300ms output quiet or unchanged screen (including styles/cursor), or explicit exit',
                'terminal_responder': 'pyte 0.8.2 / wcwidth 0.8.3 (responses only; PNG uses xterm)', 'steps': steps, 'completed_steps': completed, 'error': error,
                'exit_code_at_snapshot': proc.poll(), 'termination': 'SIGTERM after snapshot if still running'}
    finally:
        if proc.poll() is None:
            os.killpg(proc.pid, signal.SIGTERM)
            try:
                proc.wait(timeout=2)
            except subprocess.TimeoutExpired:
                os.killpg(proc.pid, signal.SIGKILL)
                proc.wait()
        os.close(master)


def steps_for(name, side):
    py = side == 'python'
    down = '\x1bOB' if py else '\x1b[B'
    provider = 'Select provider'
    terminal = 'Select terminal backend'
    gateway = 'Select platforms to configure'
    tools = 'Select an option:' if py else 'Select toolsets to enable:'
    mode = 'How would you like to set up Hermes?'
    if name == 'wizard-mode': return [(mode, None)]
    if name == 'wizard-full': return [(mode, down+'\r'), (provider, None)]
    if name == 'wizard-blank': return [(mode, down*2+'\r'), (provider, None)]
    if name == 'wizard-quick': return [(mode, '\r'), (terminal if py else provider, None)]
    if name == 'wizard-model': return [(provider, None)]
    if name == 'wizard-terminal': return [(terminal, None)]
    if name == 'wizard-local': return [(terminal, '\r'), ('@exit', None)]
    if name == 'wizard-docker': return [(terminal, down*(2 if py else 1)+'\r'), ('Docker not found', None)]
    if name == 'wizard-gateway': return [(gateway, None)]
    if name == 'wizard-gateway-empty': return [(gateway, '\r'), ('@exit', None)]
    if name == 'wizard-gateway-token': return [(gateway, ' \r'), ('Server URL', 'https://fixture.invalid\r'), ('Bot token', None)]
    if name == 'wizard-tools': return [(tools, None)]
    if name == 'wizard-tools-toggle': return ([(tools, '\r'), ('Tools for', ' '), ('Tools for', None)] if py else [(tools, ' '), (tools, None)])
    if name == 'wizard-cancel': return [(terminal, '\x1b'), ('@exit', None)]
    # Spec017 Lane 2 (W3): rendered-field evidence. Every prompt the wizard
    # draws must appear in the capture, in order, through completion/cancel.
    if name == 'wizard-model-fields':
        return [('Select provider', '\r'), ('API base URL', '\r'),
                ('Environment variable holding the API key', '\r'),
                ('Model name', 'parity-fixture\r'), ('Setup complete!', None)]
    if name == 'wizard-model-cancel':
        return [('Select provider', '\r'), ('API base URL', '\x1b'),
                ('Setup cancelled.', None), ('@exit', None)]
    if name == 'wizard-docker-image':
        return [('Select terminal backend', down + '\r'), ('Docker not found', None),
                ('Docker image', '\r'), ('Setup complete!', None)]
    if name == 'wizard-gateway-cancel':
        return [('Select platforms to configure', '\x1b'), ('Setup cancelled.', None),
                ('@exit', None)]
    if name == 'picker-empty': return [('No sessions found.', None)]
    if name == 'picker-too-small': return [('Terminal too small', None)]
    if name == 'picker-narrow': return [('Browse sessions', None)]
    if name == 'picker-normal': return [('Browse sessions', None)]
    if name == 'picker-filter': return [('Browse sessions', 'topic'), ('filter: topic', None)]
    if name == 'picker-no-match': return [('Browse sessions', 'zzzz'), ('No sessions match', None)]
    if name == 'picker-delete': return [('Browse sessions', 'd'), ('Delete', None)]
    # Spec017 W2 browse-control contracts (resize / long list / clear filter).
    # Every step's key is the action that produces the NEXT step's marker — the
    # harness only sends a key once its own marker is ready.
    if name == 'picker-resize-too-small':
        return [('Browse sessions', '@resize 60x4'),
                ('Terminal too small', '\r'), ('@exit', None)]
    if name == 'picker-resize-redraw':
        return [('Browse sessions', '@resize 80x20'),
                ('\x1b[2J', '\x1b'), ('@exit', None)]
    if name == 'picker-long-list':
        return [('Browse sessions', '\x1b[B' * 26),
                ('27/30 sessions', '\x1b[B' * 3), ('30/30 sessions', '\x1b[B'),
                ('1/30 sessions', '\x1b[A'), ('30/30 sessions', '\x1b'),
                ('@exit', None)]
    if name == 'picker-clear-filter-esc':
        return [('Browse sessions', 'sec'), ('filter: sec', '\x1b'),
                ('1/2 sessions', '\x1b'), ('@exit', None)]
    if name == 'picker-clear-filter-backspace':
        return [('Browse sessions', 'sec'), ('filter: sec', '\x7f\x7f\x7f'),
                ('1/2 sessions', '\x1b'), ('@exit', None)]
    if name.startswith('completion-'):
        text = {'completion-command':'/mod', 'completion-subcommand':'/skills ', 'completion-alternatives':'/s'}[name]
        return [('❯' if py else 'Welcome to Hermes Agent!', text+'\t\t'), (text.rstrip(), None)]
    return [('@exit', None)]


def section_for(name):
    if name.startswith('wizard-gateway'): return 'gateway'
    if name.startswith('wizard-tools'): return 'tools'
    if name.startswith('wizard-docker') or name in ('wizard-terminal', 'wizard-local', 'wizard-cancel'): return 'terminal'
    if name.startswith('wizard-model'): return 'model'
    return None


DEFAULT_PICKER_SEED = ((SID_A, 'deploy the thing'), (SID_B, 'second topic'))


def long_list_seed(count):
    """Deterministic multi-window fixture: names sort in index order and each
    session carries one user message, so every row renders with a preview."""
    return ((f'{i:08x}-0000-4000-8000-{i:012x}', f'session-{i:02d}')
            for i in range(count))


def seed_rust(home, empty=False, count=2):
    with sqlite3.connect(home/'state.db') as db:
        db.executescript('''CREATE TABLE sessions(id TEXT PRIMARY KEY,source TEXT NOT NULL,started_at REAL NOT NULL);
        CREATE TABLE messages(id INTEGER PRIMARY KEY AUTOINCREMENT,session_id TEXT NOT NULL,role TEXT NOT NULL,content TEXT,timestamp REAL NOT NULL);
        CREATE TABLE tool_calls(id TEXT PRIMARY KEY,session_id TEXT NOT NULL,turn_index INTEGER NOT NULL,tool_name TEXT NOT NULL,arguments TEXT NOT NULL,result TEXT,status TEXT NOT NULL,created_at REAL NOT NULL);''')
        if not empty:
            seed = DEFAULT_PICKER_SEED if count == 2 else tuple(long_list_seed(count))
            for i,(sid,text) in enumerate(seed):
                db.execute('INSERT INTO sessions VALUES (?,?,?)',(sid,'cli',1700000000+i))
                db.execute('INSERT INTO messages(session_id,role,content,timestamp) VALUES (?,?,?,?)',(sid,'user',text,1700000000.5+i))


def capture_side(side, binary=None, summary=None, reference=None, names=CASES,
                 widths=(100,80), timeout=15, pairs=None):
    """Record cases. By default every name is recorded at every width; pass
    `pairs` of (name, width) when the case set is width-specific (the size
    contract records 40 and 39 columns only in their own scenarios)."""
    cases = []
    plan = pairs if pairs else [(name, width) for width in widths for name in names]
    for name, width in plan:
            with tempfile.TemporaryDirectory(prefix='hermes-ui-'+side+'-') as tmp:
                home = Path(tmp)
                if name.startswith('picker') and side == 'rust':
                    seed_rust(home, name=='picker-empty',
                              count=30 if name == 'picker-long-list' else 2)
                if name.startswith('completion'):
                    (home/'config.yaml').write_text('model:\n  provider: auto\n  name: parity-fixture\n')
                if side == 'rust':
                    if name.startswith('wizard'):
                        command = [str(binary),'setup'] + ([section_for(name)] if section_for(name) else [])
                    elif name.startswith('picker'): command = [str(binary),'sessions','browse']
                    elif name.startswith('completion'): command = [str(binary),'--provider','fake']
                    else: command = [str(summary),'0' if name.endswith('zero') and not name.endswith('nonzero') else '3',str(width)]
                    extra = {}
                else:
                    command = [sys.executable,str(Path(__file__).resolve()),'python-child',str(reference),name,str(width)]
                    extra = {'PYTHONPATH': os.environ.get('PYTHONPATH','')}
                if name in ('wizard-docker', 'wizard-docker-image'):
                    (home/'empty-bin').mkdir()
                    extra['PATH'] = str(home/'empty-bin')
                result = record(command,home,width,steps_for(name,side),extra,timeout=timeout)
                # Select a recorded instant, not normalized output. Keep later
                # bytes/events too, including deliberately blocked service calls.
                if side == 'python' and name in ('wizard-docker', 'wizard-gateway-empty') and not result['error']:
                    raw = base64.b64decode(result['raw_base64'])
                    if name == 'wizard-docker':
                        pos = raw.find(b'Docker not found')
                        end = raw.find(b'\x1b[?1049h', pos) if pos >= 0 else -1
                        boundary = 'before next alternate-screen menu after Docker unavailable notice'
                    else:
                        pos = raw.find(b'Installing the gateway background service')
                        end = raw.rfind(b'\n', 0, pos)+1 if pos >= 0 else -1
                        boundary = 'after empty platform result, before deferred service orchestration'
                    if end > 0:
                        result['snapshot_end_byte'] = end
                        result['snapshot_rule'] = boundary + '; full later output retained unchanged'
                print(side,name,width,result['error'] or 'CAPTURED_NOT_REVIEWED',file=sys.stderr)
                picker_seed = ([SID_A, SID_B] if name.startswith('picker') and name != 'picker-empty'
                               and name != 'picker-long-list'
                               else [sid for sid, _ in long_list_seed(30)] if name == 'picker-long-list'
                               else [])
                cases.append({'id':f'{name}-{width}x30','scenario':name,'fixture':{'home':'isolated fresh temporary directory', 'credentials':'none supplied', 'docker_available':False if name=='wizard-docker' else 'not controlled', 'picker_seed':picker_seed, 'picker_seed_note':'long list uses deterministic session-00..session-29 names' if name=='picker-long-list' else None, 'picker_timestamp_base':1700000000 if name.startswith('picker') else None},side:result})
    return cases


def python_child(reference, name, width):
    sys.path.insert(0,str(reference))
    # External networking is forbidden; UI implementations are unmodified.
    import socket
    def deny(*args, **kwargs): raise OSError('network disabled for isolated visual evidence')
    def audit(event, args):
        if event in ('subprocess.Popen', 'os.system', 'os.posix_spawn', 'os.exec'):
            raise PermissionError('external processes disabled for isolated visual evidence')
    sys.addaudithook(audit)
    socket.socket.connect = deny
    socket.create_connection = deny
    socket.getaddrinfo = deny
    if name.startswith('wizard'):
        from types import SimpleNamespace
        from hermes_cli.setup import run_setup_wizard
        run_setup_wizard(SimpleNamespace(section=section_for(name)))
    elif name.startswith('picker'):
        from hermes_state import SessionDB
        from hermes_cli.main import _session_browse_picker
        db = SessionDB(Path(os.environ['HERMES_HOME'])/'state.db')
        if name != 'picker-empty':
            for i,(sid,text) in enumerate(((SID_A,'deploy the thing'),(SID_B,'second topic'))):
                db.create_session(sid,source='cli')
                db.append_message(sid,'user',text,timestamp=1700000000.5+i)
                with sqlite3.connect(Path(os.environ['HERMES_HOME'])/'state.db') as seed:
                    seed.execute('UPDATE sessions SET started_at=?, last_activity_at=? WHERE id=?',(1700000000+i,1700000000.5+i,sid))
        _session_browse_picker(db.list_sessions_rich(),session_db=db)
    elif name.startswith('completion'):
        from prompt_toolkit import PromptSession
        from hermes_cli.commands import SlashCommandCompleter
        # Native prompt_toolkit host of the actual REPL completer, not full CLI.
        # Prompt is host chrome; candidate labels come solely from upstream.
        PromptSession(completer=SlashCommandCompleter(),complete_while_typing=False).prompt('❯ ')
    else:
        from rich.console import Console
        from hermes_cli.banner import build_welcome_banner
        tools = [] if name=='summary-zero' else ['list_dir','read_file','write_file']
        build_welcome_banner(Console(width=width),model='parity-fixture',cwd='/fixture/demo',
                             tools=[{'function':{'name':t}} for t in tools],
                             get_toolset_for_tool=lambda name:'other',availability={},skills_by_category={})


def export(path, group):
    raw=Path(path).read_bytes()
    data=base64.b64encode(gzip.compress(raw,mtime=0)).decode()
    chunks=[data[i:i+3000] for i in range(0,len(data),3000)]
    if len(chunks)>32: raise RuntimeError('Annotation budget exceeded; never truncate')
    if group==0: print(f'::notice title=visual bundle digest::sha256={hashlib.sha256(raw).hexdigest()}; bytes={len(raw)}')
    for i in range(group*8,min((group+1)*8,len(chunks))):
        print(f'::notice title=visual bundle {i+1}/{len(chunks)}::{chunks[i]}')


def validate_bundle(bundle):
    expected = {f'{name}-{width}x30' for name in CASES for width in (100, 80)}
    assert len(bundle['cases']) == len(expected), 'missing or duplicate cases'
    assert {c['id'] for c in bundle['cases']} == expected, 'case matrix mismatch'
    assert not bundle['errors'], 'unreached UI cases'
    sides = {s for s in ('python', 'rust') if s in bundle['cases'][0]}
    assert sides, 'no captured sides'
    for c in bundle['cases']:
        assert {s for s in ('python', 'rust') if s in c} == sides
        for side in ('python', 'rust'):
            if side not in c:
                continue
            r = c[side]
            assert r['error'] is None
            assert len(r['completed_steps']) == len(r['steps'])
            raw = base64.b64decode(r['raw_base64'], validate=True)
            assert hashlib.sha256(raw).hexdigest() == r['raw_sha256']
            assert b''.join(base64.b64decode(e[1], validate=True) for e in r['events_base64']) == raw
            assert 0 < r['snapshot_end_byte'] <= len(raw)
            if c['scenario'].startswith('summary-'):
                expected_count = b'0 tools' if c['scenario']=='summary-zero' else b'3 tools'
                assert expected_count in raw[:r['snapshot_end_byte']], 'summary count not visible in captured bytes'


def main():
    mode,*args=sys.argv[1:]
    if mode=='python-child': return python_child(Path(args[0]),args[1],int(args[2]))
    if mode=='export': return export(args[0],int(args[1]))
    if mode=='check':
        validate_bundle(json.loads(Path(args[0]).read_text()))
        return
    output=Path(args[-1])
    if output.exists(): raise RuntimeError('Refusing to overwrite evidence')
    if mode=='rust':
        binary,summary=map(lambda s:Path(s).resolve(),args[:2])
        bundle={'schema':1,'status':'CAPTURED_NOT_REVIEWED','scope':'remaining UI cases',
                'rust_commit':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),
                'rustc_version':subprocess.check_output(['rustc','--version'],text=True).strip(),
                'cargo_version':subprocess.check_output(['cargo','--version'],text=True).strip(),
                'rust_worktree_diff_sha256':hashlib.sha256(subprocess.check_output(['git','diff','--binary','--','*.rs'])).hexdigest(),
                'binary_sha256':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in (binary,summary)},
                'python_reference':UPSTREAM,
                'cases':capture_side('rust',binary=binary,summary=summary)}
    elif mode=='python':
        reference=Path(args[0]).resolve()
        assert (reference/'.visual-evidence-reference').read_text().strip()==UPSTREAM
        bundle={'schema':1,'python_reference':UPSTREAM,'python_version':sys.version,
                'source_sha256':{str(p.relative_to(reference)):hashlib.sha256(p.read_bytes()).hexdigest()
                                  for p in reference.rglob('*.py')},
                'cases':capture_side('python',reference=reference)}
    elif mode=='pair':
        bundle=json.loads(Path(args[0]).read_text());py=json.loads(Path(args[1]).read_text())
        assert bundle['python_reference']==py['python_reference']
        lookup={c['id']:c['python'] for c in py['cases']}
        for case in bundle['cases']: case['python']=lookup.pop(case['id'])
        assert not lookup
        bundle['python_version']=py['python_version'];bundle['python_source_sha256']=py['source_sha256']
    else: raise ValueError(mode)
    bundle['errors']=[{'id':c['id'],'side':s,'error':c[s]['error']} for c in bundle['cases'] for s in ('python','rust') if s in c and c[s]['error']]
    bundle['status'] = 'CAPTURE_INCOMPLETE' if bundle['errors'] else 'CAPTURED_NOT_REVIEWED'
    bundle['capture_driver_sha256'] = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    output.parent.mkdir(parents=True,exist_ok=True)
    output.write_text(json.dumps(bundle,indent=2)+'\n')


if __name__=='__main__': main()
