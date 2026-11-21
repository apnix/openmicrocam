"""The camera from the laptop, for `tools/cam/cam` and the testers in `tools/cam/test/`: its shell (USB telnet, the
command's output and exit code only), microcamd's control protocol (TCP 7700, one JSON line each way), the link, the
laptop's hold on the video node and the host's hooks. Addresses, login, paths and hooks: tools/hostconf.py."""
import atexit, json, os, re, signal, socket, statistics, subprocess, sys, time

REPO = os.path.abspath(os.path.join(os.path.dirname(os.path.realpath(__file__)), '..', '..'))
sys.path.insert(0, os.path.join(REPO, 'tools'))
import hostconf  # noqa: E402

LOGS = hostconf.get('MICROCAM_LOGS')
CAM = hostconf.get('MICROCAM_CAMERA')
HOST = hostconf.get('MICROCAM_HOST')
MICROCAM_PORT = 7700
VIDEO = hostconf.get('MICROCAM_VIDEO')


class Gone(Exception):
    """The camera does not answer."""


# ---- the shell ----------------------------------------------------------------------------------------------------

IAC, DONT, DO, WONT, WILL, SB, SE = 255, 254, 253, 252, 251, 250, 240


class Shell:
    """One logged-in telnet session, prompts off; `sh` below runs a command in it."""

    def __init__(self, deadline):
        self.deadline = deadline
        try:
            self.s = socket.create_connection((CAM, 23), timeout=max(0.2, deadline - time.time()))
        except OSError as e:
            raise Gone(f'telnet: {e}') from e
        self.buf = b''
        user, password = hostconf.get('MICROCAM_USER'), hostconf.get('MICROCAM_PASSWORD')
        if not password:
            raise Gone('telnet: MICROCAM_PASSWORD is set empty: unset it for the stock login (docs/TOOLS.md)')
        self._until(rb'login: ?$')
        self.s.sendall(user.encode() + b'\r\n')
        self._until(rb'assword: ?$')
        self.s.sendall(password.encode() + b'\r\n')
        self._until(rb'# ?$')
        # no prompts from here on; the markers below are typed with '' inside so the echo never matches them
        self.s.sendall(b"PS1=; PS2=; stty -echo; echo __R''DY__\r\n")
        self._until(rb'__RDY__\r?\n')

    def _recv(self):
        left = self.deadline - time.time()
        if left <= 0:
            raise TimeoutError('timed out')
        self.s.settimeout(left)
        data = self.s.recv(65536)
        if not data:
            raise Gone('telnet: the camera closed the session')
        clean, reply, i = bytearray(), bytearray(), 0
        while i < len(data):
            b = data[i]
            if b == IAC and i + 1 < len(data):
                c = data[i + 1]
                if c in (DO, WILL) and i + 2 < len(data):
                    reply += bytes([IAC, WONT if c == DO else DONT, data[i + 2]])
                    i += 3
                elif c in (DONT, WONT) and i + 2 < len(data):
                    i += 3
                elif c == SB:
                    j = data.find(bytes([IAC, SE]), i)
                    i = j + 2 if j != -1 else len(data)
                else:
                    i += 2
                continue
            clean.append(b)
            i += 1
        if reply:
            self.s.sendall(bytes(reply))
        self.buf += bytes(clean)

    def _until(self, pattern):
        rx = re.compile(pattern, re.M)
        while not (m := rx.search(self.buf)):
            self._recv()
        out, self.buf = self.buf[:m.start()], self.buf[m.end():]
        return out

    def close(self):
        try:
            self.s.close()
        except OSError:
            pass


def sh(cmd, timeout=10.0, script=None, log=True):
    """(exit code, output) of one command on the camera; 124 and what came when it ran out of time."""
    deadline = time.time() + timeout
    t0 = time.strftime('%H:%M:%S')
    sess = None
    try:
        sess = Shell(deadline)
        body = f"sh <<'__MICROCAM_EOF__'\n{script.rstrip()}\n__MICROCAM_EOF__" if script is not None else f'(\n{cmd}\n)'
        sess.s.sendall(f"echo __B''EG__; {body}\necho __E''ND__$?\n".replace('\n', '\r\n').encode())
        sess._until(rb'^__BEG__\r?\n')
        rx = re.compile(rb'^__END__(\d+)\r?$', re.M)
        while not (m := rx.search(sess.buf)):
            sess._recv()
        rc, out = int(m.group(1)), sess.buf[:m.start()]
    except TimeoutError:
        rc, out = 124, (sess.buf if sess else b'')
    finally:
        if sess:
            sess.close()
    text = out.decode('utf-8', 'replace').replace('\r', '')
    if log:  # a transcript of the camera's shell
        try:
            os.makedirs(LOGS, exist_ok=True)
            with open(os.path.join(LOGS, f'telnet-{time.strftime("%F")}.log'), 'a') as f:
                f.write(f'\n\033[1;36m[{t0} >>> {cmd if script is None else "(script)"}]\033[0m\n{text}'
                        + ('' if rc == 0 else f'[exit {rc}]\n'))
        except OSError:
            pass
    return rc, text


# ---- microcamd --------------------------------------------------------------------------------------------------------

def microcam(req, timeout=2.0):
    """One request to microcamd and its reply (a dict)."""
    try:
        with socket.create_connection((CAM, MICROCAM_PORT), timeout=timeout) as s:
            s.settimeout(timeout)
            s.sendall((json.dumps(req) + '\n').encode())
            buf = b''
            while True:
                chunk = s.recv(1 << 20)
                if not chunk:
                    raise Gone('microcamd closed the connection')
                buf += chunk
                while b'\n' in buf:
                    line, buf = buf.split(b'\n', 1)
                    if not line.startswith(b'{"event"'):
                        return json.loads(line)
    except (OSError, ValueError) as e:
        raise Gone(f'microcamd: {e}') from e


def status(timeout=2.0):
    r = microcam({'cmd': 'status'}, timeout)
    if not r.get('ok'):
        raise Gone(f"microcamd: {r.get('error')}")
    return r['status']


# ---- the laptop's side --------------------------------------------------------------------------------------------

def link():
    """The camera's RNDIS interface on the laptop, or None; one with a carrier first (an earlier boot's may linger)."""
    out = subprocess.run(['ip', '-br', 'link'], capture_output=True, text=True).stdout
    ifs = [l.split() for l in out.splitlines() if l.startswith('enx')]
    live = [f[0] for f in ifs if 'NO-CARRIER' not in f[-1]]
    return (live or [f[0] for f in ifs] or [None])[0]


def ping(timeout=1):
    return subprocess.run(['ping', '-c1', f'-W{timeout}', CAM], capture_output=True).returncode == 0


def net_up():
    """cam-net.sh: the laptop's address on a new interface name (after every camera reboot)."""
    return subprocess.run(['sh', os.path.join(REPO, 'tools/cam/cam-net.sh')], capture_output=True, timeout=15).returncode == 0


def holders():
    """[(pid, command)] of laptop processes holding the camera's video node."""
    try:
        dev = os.path.realpath(VIDEO)
    except OSError:
        return []
    if not os.path.exists(dev):
        return []
    r = subprocess.run(['fuser', dev], capture_output=True, text=True)
    out = []
    for pid in r.stdout.split():
        pid = int(re.sub(r'\D', '', pid) or 0)
        try:
            cmd = open(f'/proc/{pid}/cmdline', 'rb').read().replace(b'\0', b' ').decode(errors='replace').strip()
        except OSError:
            cmd = '?'
        out.append((pid, cmd))
    return out


# ---- a program as the camera keeps it ---------------------------------------------------------------------------

def pack_xz(src, dst):
    """The camera's file of a built program (microcamd.xz, microcam-menu.xz) at `dst`: an .xz is taken as it is, a binary
    is compressed with a 1 MiB dictionary, the most busybox unxz accepts. Used by `cam deploy` and `install.py`."""
    if src.endswith('.xz'):
        if os.path.abspath(src) != os.path.abspath(dst):
            subprocess.run(['cp', src, dst], check=True)
        return dst
    with open(dst + '.tmp', 'wb') as f:
        subprocess.run(['xz', '-9e', '--lzma2=preset=9e,dict=1MiB', '-c', src], stdout=f, check=True)
    os.replace(dst + '.tmp', dst)
    return dst


# ---- the host's hooks (docs/TOOLS.md) ---------------------------------------------------------------------------

def _hook(name, timeout):
    cmd = hostconf.get(name)
    if not cmd:
        return None
    try:
        return subprocess.run(['sh', '-c', cmd], capture_output=True, text=True, timeout=timeout, env=hostconf.environ())
    except subprocess.TimeoutExpired:
        return subprocess.CompletedProcess(cmd, 124, '', f'{name}: no end within {timeout} s')


def stream_release():
    """The host's stream-release hook before a tool opens the video node or reboots the camera: whether it freed a
    stream that `stream_restore` must give back."""
    r = _hook('MICROCAM_HOOK_STREAM_RELEASE', 30)
    return r is not None and r.returncode == 0


def stream_restore(released):
    """The host's stream-restore hook when `stream_release` freed a stream: None when nothing was freed, else whether
    the stream is back."""
    if not released:
        return None
    r = _hook('MICROCAM_HOOK_STREAM_RESTORE', 150)
    return r is not None and r.returncode == 0


def change_refused():
    """The host's change hook before a command that may change the camera: None when the command may run, else the
    hook's reason for refusing it."""
    r = _hook('MICROCAM_HOOK_CHANGE', 10)
    if r is None or r.returncode == 0:
        return None
    return (r.stdout + r.stderr).strip() or f'refused by MICROCAM_HOOK_CHANGE (exit {r.returncode})'


def state_extra():
    """The host's state hook: (pids allowed to hold the video node, anomaly lines, info lines)."""
    r = _hook('MICROCAM_HOOK_STATE', 3)
    allow, warn, info = set(), [], []
    if r is None:
        return allow, warn, info
    if r.returncode != 0:
        return allow, [f'⚠ state hook failed (exit {r.returncode}): {(r.stderr.strip().splitlines() or [""])[-1][:60]}'], info
    for l in r.stdout.splitlines():
        if l.startswith('allow '):
            allow |= {int(p) for p in l.split()[1:] if p.isdigit()}
        elif l.startswith('⚠'):
            warn.append(l)
        elif l.strip():
            info.append(l)
    return allow, warn, info


# ---- for the testers ----------------------------------------------------------------------------------------------

class Menu:
    """A control connection with `seq`s: replies by number, microcamd's events kept with their time."""

    def __init__(self, timeout=10):
        import threading
        self.s = socket.create_connection((CAM, MICROCAM_PORT), timeout=timeout)
        self.s.settimeout(None)
        self.s.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
        self.f = self.s.makefile('rw')
        self.seq, self.replies, self.events = 0, {}, []
        self.cv, self.wl = threading.Condition(), threading.Lock()
        threading.Thread(target=self._read, daemon=True).start()

    def _read(self):
        try:
            for l in self.f:
                m = json.loads(l)
                with self.cv:
                    if 'event' in m:
                        self.events.append((time.monotonic(), m))
                    else:
                        self.replies[m.get('seq')] = m
                    self.cv.notify_all()
        except (OSError, ValueError):
            pass

    def send(self, req):
        with self.wl:
            self.seq += 1
            self.f.write(json.dumps(dict(req, seq=self.seq), ensure_ascii=False) + '\n')
            self.f.flush()
            return self.seq

    def ask(self, req, t=10):
        s = self.send(req)
        with self.cv:
            self.cv.wait_for(lambda: s in self.replies, t)
            return self.replies.get(s) or {}

    def event(self, pred, since, t=15):
        """The first event after `since` (monotonic) for which `pred` holds, None after `t` s."""
        with self.cv:
            hit = lambda: next((m for at, m in self.events if at >= since and pred(m)), None)
            self.cv.wait_for(lambda: hit() is not None, t)
            return hit()

    def close(self):
        """The connection closed for microcamd too (the file wrapper holds the socket open until it is closed itself).
        The socket shut down first: the reader thread blocked in the file holds its lock, and closing the file waits
        for that lock until something arrives."""
        try:
            self.s.shutdown(socket.SHUT_RDWR)
        except OSError:
            pass
        for x in (self.f, self.s):
            try:
                x.close()
            except OSError:
                pass


def heap_free():
    """The media heap's free MiB (`mem --brief`, else the full report)."""
    lines = microcam({'cmd': 'mem', 'brief': True}, 4).get('lines') or []
    text = '\n'.join(lines)
    m = re.search(r'heap free ([\d.]+)', text) or re.search(r'media heap [\d.]+ MiB: free ([\d.]+)', text)
    return float(m[1]) if m else None


def menu_held():
    """The menu's share held now, MiB."""
    lines = microcam({'cmd': 'mem', 'brief': True}, 4).get('lines') or []
    m = re.search(r'menu ([\d.]+) of', '\n'.join(lines))
    return float(m[1]) if m else None


def account():
    """microcamd's `account`, MiB: the video priced, the heap's listing held, free, microcamd's share and what it uses."""
    acc = microcam({'cmd': 'account'}, 6).get('account') or {}
    m = lambda b: round(b / (1 << 20), 1)
    return {'priced': m(sum(b for _, b in acc.get('priced', []))), 'held': m(sum(b for _, b in acc.get('held', []))),
            'free': m(acc.get('free', 0)), 'share': m(acc.get('share', 0)), 'used': m(acc.get('used', 0))}


def mp4_gaps(path):
    """The MP4's frames, frame period ms, gaps over 1.5 periods as (at s, ms) — from its boxes alone, read by ranges over
    HTTP (a 10-min 4K recording is 1.5 GB: the samples are never pulled). The recorder's fragmented MP4 (`mp4.rs`): one
    `moof` + `mdat` a GOP, each `trun` sample with its duration."""
    import struct, urllib.request
    d, f = os.path.split(path)
    sh(f'killall httpd 2>/dev/null; httpd -p 8090 -h {d}', 10)
    url = f'http://{CAM}:8090/{f}'

    def get(at, n):
        r = urllib.request.Request(url, headers={'Range': f'bytes={at}-{at + n - 1}'})
        with urllib.request.urlopen(r, timeout=20) as resp:
            return resp.read()

    def boxes(b):
        k = 0
        while k + 8 <= len(b):
            n, t = struct.unpack('>I4s', b[k:k + 8])
            yield t, b[k + 8:k + n]
            k += n
    try:
        size = int(sh(f'stat -c %s {path}', 10)[1].strip())
        at, scale, ts = 0, 90000, []
        while at + 8 <= size:
            n, t = struct.unpack('>I4s', get(at, 8))
            if n < 8:
                break
            if t in (b'moov', b'moof'):
                body = get(at + 8, n - 8)
                if t == b'moov':
                    m = body.find(b'mdhd')
                    if m >= 0:
                        scale = struct.unpack('>I', body[m + 16:m + 20])[0]
                for tt, traf in boxes(body):
                    if tt != b'traf':
                        continue
                    base, durs = None, []
                    for bt, bb in boxes(traf):
                        if bt == b'tfdt':
                            base = struct.unpack('>Q' if bb[0] == 1 else '>I', bb[4:12] if bb[0] == 1 else bb[4:8])[0]
                        elif bt == b'trun':
                            flags = int.from_bytes(bb[1:4], 'big')
                            cnt = struct.unpack('>I', bb[4:8])[0]
                            k = 8 + (4 if flags & 1 else 0) + (4 if flags & 4 else 0)
                            per = 4 * bin(flags & 0xf00).count('1')
                            for _ in range(cnt):
                                durs.append(struct.unpack('>I', bb[k:k + 4])[0] if flags & 0x100 else 0)
                                k += per
                    for dur in durs:
                        ts.append(base / scale)
                        base += dur
            at += n
    finally:
        sh('killall httpd', 10)
    ts.sort()
    if len(ts) < 2:
        return len(ts), None, []
    steps = [b - a_ for a_, b in zip(ts, ts[1:])]
    period = statistics.median(steps)
    return len(ts), round(period * 1000, 1), [(round(t, 2), round(s * 1000)) for t, s in zip(ts, steps) if s > period * 1.5]


# ---- what a tester changes, put back -------------------------------------------------------------------------------
# A control a tester sets (`change`), the card reader mode among them, goes first into a journal on the
# laptop with what they were; when the tester ends — at its end, by an exception, Ctrl-C, SIGTERM, SIGHUP, SIGALRM — they
# are put back, the last changed first, and the journal emptied. What cannot be put back then (the link lost, the
# process killed outright) stays in the journal and is put back when the next tester starts (`Report`).

JOURNAL = os.path.join(LOGS, 'to-restore.json')
_restore_armed = False


def _journal():
    try:
        with open(JOURNAL) as f:
            return json.load(f)
    except (OSError, ValueError):
        return {'controls': {}, 'order': []}


def _write_journal(j):
    if not j['controls']:
        if os.path.exists(JOURNAL):
            os.remove(JOURNAL)
        return
    os.makedirs(LOGS, exist_ok=True)
    with open(JOURNAL + '.tmp', 'w') as f:
        json.dump(j, f)
    os.replace(JOURNAL + '.tmp', JOURNAL)


def _exit_on(signum, _frame):
    sys.exit(128 + signum)


def _restore_on_exit():
    """The journal put back however this process ends (but SIGKILL)."""
    global _restore_armed
    if _restore_armed:
        return
    _restore_armed = True
    atexit.register(restore)
    for s in (signal.SIGTERM, signal.SIGHUP, signal.SIGALRM):
        if signal.getsignal(s) in (signal.SIG_DFL, None):
            signal.signal(s, _exit_on)


def _set(name, value, timeout=20):
    """`set` of control `name`, returned once it is built: microcamd answers a built control's `set` at once and builds
    it after (its `control` event, or `failed`), its value meanwhile still the one before — a tester that went on would
    leave that build behind it, and the next one would read the old value as the camera's (and put it back after)."""
    c = Menu()
    try:
        snap = c.ask({'cmd': 'watch'}, timeout).get('snapshot') or {}
        now = next((x['value'] for x in snap.get('controls', []) if x['name'] == name), None)
        since = time.monotonic()
        r = c.ask({'cmd': 'set', 'name': name, 'value': value}, timeout)
        v = r.get('value', value)
        if not r.get('ok') or now == v:
            return r
        built = lambda m: m.get('event') == 'control' and m['control']['name'] == name and m['control']['value'] == v
        m = c.event(lambda m: built(m) or (m.get('event') == 'failed' and m.get('what') == {'control': name}), since, timeout)
        if m is None:
            return {'ok': False, 'error': f'{name} not built in {timeout} s'}
        return r if m['event'] == 'control' else {'ok': False, 'code': m.get('code'), 'error': m.get('error')}
    finally:
        c.close()


def change(name, value, timeout=20):
    """Control `name` set to `value`: microcamd's reply. Its value before is written into the journal first, so it is
    put back when the tester ends."""
    j = _journal()
    if name not in j['controls']:
        snap = microcam({'cmd': 'watch'}, timeout).get('snapshot') or {}
        was = next((c['value'] for c in snap.get('controls', []) if c['name'] == name), None)
        if was is None:
            return {'ok': False, 'error': f'no control {name}'}
        j['controls'][name] = was
        j['order'].append(name)
        _write_journal(j)
    _restore_on_exit()
    return _set(name, value, timeout)


def restore():
    """Everything in the journal put back, the controls last changed first; what cannot be (the camera gone, a control
    refused) stays for the next run and is said on stderr."""
    j = _journal()
    if not j['controls']:
        return
    try:
        for name in reversed(list(j['order'])):
            r = {}
            for _ in range(40):
                r = _set(name, j['controls'][name], 20)
                # one build of a control at a time: the tester's last may still be building
                if r.get('code') != 'busy':
                    break
                time.sleep(0.5)
            if r.get('ok'):
                del j['controls'][name]
                j['order'].remove(name)
            else:
                print(f'camlib: {name} not put back to {j["controls"][name]}: {r.get("error")}', file=sys.stderr)
    except (Gone, OSError) as e:
        print(f'camlib: the camera does not answer ({e}); what is left is put back at the next tester\'s start', file=sys.stderr)
    _write_journal(j)
    if j['controls']:
        print(f'camlib: left to put back: {JOURNAL}', file=sys.stderr)


def record_start(t=15):
    """`record`, then waited for until the recording runs (its `running` state): None, or why not — its refusal, its
    `failed`, or the wait over."""
    c = Menu()
    try:
        c.ask({'cmd': 'watch'})
        since = time.monotonic()
        r = c.ask({'cmd': 'record'})
        if not r.get('ok'):
            return f'{r.get("code")}: {r.get("error")}'
        runs = lambda m: m.get('event') == 'running' and ((m['running'].get('record') or {}).get('state') == 'running')
        m = c.event(lambda m: runs(m) or (m.get('event') == 'failed' and m.get('what') == 'record'), since, t)
        if m is None:
            return f'not running after {t} s'
        return None if m.get('event') == 'running' else f'{m.get("code")}: {m.get("error")}'
    finally:
        c.close()


def record_stop(t=30):
    """`record-stop`, then waited for until the file is closed (`recorded`): (file, None), or (None, why not)."""
    c = Menu()
    try:
        c.ask({'cmd': 'watch'})
        since = time.monotonic()
        r = c.ask({'cmd': 'record-stop'})
        if not r.get('ok'):
            return None, f'{r.get("code")}: {r.get("error")}'
        m = c.event(lambda m: m.get('event') == 'recorded' or (m.get('event') == 'failed' and m.get('what') in ('record', 'record-stop')), since, t)
        if m is None:
            return None, f'not stopped after {t} s'
        return (m['file'], None) if m.get('event') == 'recorded' else (None, f'{m.get("code")}: {m.get("error")}')
    finally:
        c.close()


def press_photo():
    """A full-size photo into the card's next PHOTO/P#####.jpg (`photo`): microcamd's reply, given at once (its outcome,
    `photo-saved` or `failed`, comes as an event)."""
    return microcam({'cmd': 'photo'}, 5)


def disp_handed(pool):
    """(canvases, DISP's canvases handed so far) of pool `pool` (/proc/microcam_canvas), None without the pool."""
    _, text = sh('cat /proc/microcam_canvas', 10, log=False)
    cur = None
    for l in text.splitlines():
        if m := re.match(r'pool (\d+) \S+ rows \d+ size \d+ canvases (\d+) open', l):
            cur = (int(m[1]), int(m[2]))
        elif l.startswith('pool '):
            cur = None
        elif cur and cur[0] == pool and (m := re.match(r'\s+reader 15 \S+ handed (\d+)', l)):
            return cur[1], int(m[1])
    return None


def log_mark():
    return int(microcam({'cmd': 'log-since', 'from': 2 ** 64 - 1}).get('message') or 0)


def log_since(n):
    return microcam({'cmd': 'log-since', 'from': n}, 5).get('lines') or []


def bad_lines(lines):
    """microcamd's own warnings and errors (the vendor's kernel lines left out)."""
    return [l for l in lines if re.search(r' (WARN|ERROR) ', l) and ' kernel ' not in l]


def host_stream(codec, size, fps, frames, secs):
    """A host stream from the laptop: (frames received, ffmpeg's last error line). `timeout -k`: ffmpeg ignores TERM
    while no frame comes."""
    fmt = {'mjpeg': 'mjpeg', 'h264': 'h264', 'h265': 'hevc'}[codec]
    r = subprocess.run(['timeout', '-k', '2', str(secs), 'ffmpeg', '-loglevel', 'error', '-f', 'v4l2', '-input_format', fmt,
                        '-video_size', size, '-framerate', str(fps), '-i', VIDEO, '-frames:v', str(frames), '-f', 'framecrc', '-'],
                       capture_output=True, text=True)
    n = sum(1 for l in r.stdout.splitlines() if l.startswith('0,'))
    err = (r.stderr.strip().splitlines() or [''])[-1][-120:]
    if r.returncode in (124, 137):
        err = f'timed out after {secs} s' + (f' ({err})' if err else '')
    return n, err


class Report:
    """One compact line a case (PASS / FAIL / ----), or with --json one JSON object a case; the details into
    $MICROCAM_LOGS/<name>-<date>.log."""

    def __init__(self, name, as_json=False):
        # what an earlier tester left changed (it died with the link down) put back first
        restore()
        self.name, self.as_json, self.fails = name, as_json, 0
        os.makedirs(LOGS, exist_ok=True)
        self.path = os.path.join(LOGS, f'{name}-{time.strftime("%F")}.log')
        self.log = open(self.path, 'a')
        self.log.write(f'\n== {time.strftime("%T")} {name}\n')

    def detail(self, text):
        self.log.write(text.rstrip() + '\n')
        self.log.flush()

    def case(self, case, ok, note='', **nums):
        """ok: True / False / None (measured, no threshold)."""
        self.fails += ok is False
        if self.as_json:
            print(json.dumps(dict(case=case, ok=ok, note=note, **nums), ensure_ascii=False), flush=True)
        else:
            mark = {True: 'PASS', False: 'FAIL', None: '----'}[ok]
            vals = ' '.join(f'{k}={v}' for k, v in nums.items())
            print(f'{mark} {case:<26} {vals}' + (f'  — {note}' if note else ''), flush=True)
        self.detail(f'{case}: ok={ok} {nums} {note}')
