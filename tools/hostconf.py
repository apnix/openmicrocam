#!/usr/bin/env python3
"""The host settings of the tools: one set of environment variables, the build's (docs/TOOLS.md «Settings») and the camera's,
listed in docs/TOOLS.md. Each is read from the environment, else from the config file, else its default below.

The config file is a shell file of `NAME=value` lines, sourced by sh in its own directory (so `$PWD`, `$HOME` and
command substitution work): $MICROCAM_CONFIG, else `microcam.env` at the repository's top (git-ignored).
usage: hostconf.py           the settings as NAME=value lines (the password shown only as stock or set)
       hostconf.py --sh      `export` lines for a shell script: eval "$(python3 tools/hostconf.py --sh)"
"""
import os, shlex, subprocess, sys

REPO = os.path.abspath(os.path.join(os.path.dirname(os.path.realpath(__file__)), '..'))
STATE = os.environ.get('XDG_STATE_HOME') or os.path.join(os.path.expanduser('~'), '.local', 'state')

# The stock firmware's root login, the same on every YIZHAN 4K camera: the device's published factory default, not a
# secret. A camera whose password was changed takes MICROCAM_USER / MICROCAM_PASSWORD instead.
STOCK_USER, STOCK_PASSWORD = 'root', 'ws123321'

DEFAULTS = {
    'MICROCAM_CAMERA': '192.168.1.100',  # the camera's address on its USB network
    'MICROCAM_HOST': '192.168.1.2',  # the laptop's address there
    'MICROCAM_USER': STOCK_USER,  # the camera's shell login
    'MICROCAM_PASSWORD': STOCK_PASSWORD,
    'MICROCAM_VIDEO': '/dev/v4l/by-id/usb-Linux_Foundation_Webcam_gadget-video-index0',
    'MICROCAM_LOGS': os.path.join(STATE, 'openmicrocam'),
    'MICROCAM_STAGE': os.path.join(REPO, 'stage'),
    'MICROCAM_SERIAL': '/dev/ttyUSB0',
    'MICROCAM_SERIAL_BAUD': '115200',
    'MICROCAM_PROGRESS': '',
    'MICROCAM_HOOK_STREAM_RELEASE': '',
    'MICROCAM_HOOK_STREAM_RESTORE': '',
    'MICROCAM_HOOK_STATE': '',
    'MICROCAM_HOOK_CHANGE': '',
    # the build's inputs, made by docs/INSTALL.md's steps in the git-ignored work/ here
    'MICROCAM_SYSROOT': os.path.join(REPO, 'work', 'dump'),  # the camera's dump, unpacked (install.py dump)
    'MICROCAM_LINK_LIBS': os.path.join(REPO, 'work', 'build', 'linklibs'),  # mk-linklibs.sh
    'KTREE': os.path.join(REPO, 'work', 'build', 'linux-4.9.227'),  # prepare-ktree.sh
}


def config_file():
    path = os.environ.get('MICROCAM_CONFIG') or os.path.join(REPO, 'microcam.env')
    return path if os.path.isfile(path) else None


def _from_file(path):
    r = subprocess.run(['sh', '-c', 'set -a; . "$1" >&2 || exit 1; env -0', 'sh', os.path.abspath(path)],
                       cwd=os.path.dirname(os.path.abspath(path)), capture_output=True)
    if r.returncode != 0:
        sys.exit(f'hostconf: {path} failed: {r.stderr.decode(errors="replace").strip()}')
    out = {}
    for item in r.stdout.split(b'\0'):
        k, _, v = item.decode(errors='replace').partition('=')
        if k in DEFAULTS:
            out[k] = v
    return out


_conf = None


def conf():
    """{name: value} of every setting."""
    global _conf
    if _conf is None:
        path = config_file()
        filed = _from_file(path) if path else {}
        _conf = {k: os.environ[k] if k in os.environ else filed.get(k, d) for k, d in DEFAULTS.items()}
    return _conf


def get(name):
    return conf()[name]


def environ():
    """The process environment with the settings in it: for a build or a tool started from Python."""
    return {**os.environ, **{k: v for k, v in conf().items() if v != ''}}


def main():
    c = conf()
    if sys.argv[1:] == ['--sh']:
        for k, v in c.items():
            print(f'export {k}={shlex.quote(v)}')
        return
    print(f'# config: {config_file() or "none (defaults and the environment)"}')
    for k, v in c.items():
        print(f'{k}={("(stock)" if v == STOCK_PASSWORD else "(set)" if v else "(unset)") if k == "MICROCAM_PASSWORD" else v}')


if __name__ == '__main__':
    main()
