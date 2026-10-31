#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Shin315
# SPDX-License-Identifier: MIT

"""The one-way export of the community edition (community/README.md).

Builds the community tree from a commit of this repository (main by
default), checks it for leaks, and can build and test it. Pushing to the
public repository is written but turned off until the token and the
repository move are confirmed (PUSH_ENABLED below).

    community/export.py --out DIR              the tree of main, then the leak check
    community/export.py --out DIR --worktree   the files as they are now, committed or not
    community/export.py --out DIR --build      also npm ci, vitest, the frontend
                                               build, clippy and cargo test in DIR
    community/export.py --out DIR --push       refused while PUSH_ENABLED is False

What leaves the tree, in this order:

1. the paths in community/private-paths.txt (globs; a directory ends in /);
2. in every text file, the lines from one holding `@private:begin` to the
   next holding `@private:end`, both included;
3. every `.rs` file whose first lines hold `#![cfg(feature = "author-tools")]`;
4. in every `.rs` file, each item under `#[cfg(feature = "author-tools")]`
   (with the doc comments, comments and attributes right above it), and the
   attribute alone from `#![cfg(not(feature = "author-tools"))]` or
   `#[cfg(not(feature = "author-tools"))]`;
5. in the JSON files community/json-keys.txt names, the keys it names (the
   test fixtures' keys only the private part reads);
6. then the files under community/overlay/ are copied over the tree, and
   the repository's URL is replaced by --public-url.

The leak check then reads every path and every text line of the tree
against community/leak-patterns.txt and fails on any match.

Python 3.9 and its standard library only: macOS's own python3 runs it.
"""

import argparse
import fnmatch
import io
import json
import os
import re
import shutil
import subprocess
import sys
import tarfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)

# Stays False until firstmate confirms the token and the repository move
# (the proposal's steps 3 and 5). No workflow runs this script with --push.
PUSH_ENABLED = False

FEATURE = 'author-tools'
DEFAULT_URL = 'https://github.com/shin315/quota-station'
# The URL the private repository's files name, as they are written today.
SOURCE_URL = 'https://github.com/shin315/quota-station'

BEGIN = '@private:begin'
END = '@private:end'

CFG_ON = re.compile(r'#\[cfg\((?:feature = "%s"|all\(feature = "%s"[^\]]*\))\)\]' % (FEATURE, FEATURE))
CFG_OFF = re.compile(r'#!?\[cfg\(not\(feature = "%s"\)\)\]' % FEATURE)
CFG_FILE = '#![cfg(feature = "%s")]' % FEATURE


class ExportError(Exception):
    pass


# --- reading the source ---------------------------------------------------


def git(*args, cwd=ROOT):
    return subprocess.run(['git', *args], cwd=cwd, check=True, capture_output=True).stdout


def source_files(ref, worktree):
    """Each tracked path and its bytes: of `ref`, or as the files are now."""
    if worktree:
        out = git('ls-files', '-z', '--cached', '--others', '--exclude-standard')
        for path in sorted(p for p in out.decode().split('\0') if p):
            full = os.path.join(ROOT, path)
            if os.path.isfile(full) and not os.path.islink(full):
                with open(full, 'rb') as f:
                    yield path, f.read(), os.access(full, os.X_OK)
        return
    archive = git('archive', '--format=tar', ref)
    with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
        for member in tar.getmembers():
            if member.isfile():
                yield member.name, tar.extractfile(member).read(), bool(member.mode & 0o111)


def read_list(name):
    with open(os.path.join(HERE, name), encoding='utf-8') as f:
        return [line.rstrip('\n') for line in f if line.strip() and not line.startswith('#')]


def is_private(path, globs):
    for g in globs:
        if g.endswith('/'):
            if path.startswith(g):
                return True
        elif fnmatch.fnmatchcase(path, g):
            return True
    return False


# --- markers --------------------------------------------------------------


def strip_markers(text, path):
    out = []
    inside = None
    for n, line in enumerate(text.split('\n'), 1):
        if BEGIN in line:
            if inside is not None:
                raise ExportError('%s:%d: %s inside the block opened on line %d' % (path, n, BEGIN, inside))
            inside = n
            continue
        if END in line:
            if inside is None:
                raise ExportError('%s:%d: %s without %s' % (path, n, END, BEGIN))
            inside = None
            continue
        if inside is None:
            out.append(line)
    if inside is not None:
        raise ExportError('%s:%d: %s never closed' % (path, inside, BEGIN))
    return '\n'.join(out)


# --- Rust items under the feature ----------------------------------------

OPEN = '([{'
CLOSE = ')]}'
IDENT = re.compile(r'[A-Za-z0-9_]')


def skip_string(text, i):
    """The index after the string, char or raw string literal starting at `i`, or None."""
    m = re.compile(r'b?r(#*)"').match(text, i)
    if m:
        end = text.find('"' + m.group(1), m.end())
        if end < 0:
            raise ExportError('a raw string never ends')
        return end + 1 + len(m.group(1))
    j = i + 1 if text.startswith('b"', i) or text.startswith("b'", i) else i
    if text[j] == '"':
        k = j + 1
        while text[k] != '"':
            k += 2 if text[k] == '\\' else 1
        return k + 1
    if text[j] == "'":
        # A char literal ('a', '\n', '\u{1F600}'), not a lifetime ('a).
        m = re.compile(r"'(?:\\(?:u\{[0-9A-Fa-f]+\}|x[0-9A-Fa-f]{2}|.)|[^\\'])'").match(text, j)
        if m:
            return m.end()
    return None


def skip_trivia(text, i):
    """Past whitespace and comments."""
    while i < len(text):
        if text[i].isspace():
            i += 1
        elif text.startswith('//', i):
            end = text.find('\n', i)
            i = len(text) if end < 0 else end + 1
        elif text.startswith('/*', i):
            end = text.find('*/', i)
            if end < 0:
                raise ExportError('a block comment never ends')
            i = end + 2
        else:
            break
    return i


def item_end(text, i):
    """The end of the item, statement, field, argument or match arm at `i`."""
    depth = 0
    angle = 0
    while True:
        i = skip_trivia(text, i)
        if i >= len(text):
            return i
        c = text[i]
        if c in '"\'' or (c in 'br' and skip_string(text, i) is not None and (i == 0 or not IDENT.match(text[i - 1]))):
            after = skip_string(text, i)
            if after is not None:
                i = after
                continue
            i += 1
            continue
        if c in OPEN:
            depth += 1
        elif c in CLOSE:
            if depth == 0:
                return i
            depth -= 1
            if depth == 0 and c == '}':
                j = skip_trivia(text, i + 1)
                if j < len(text) and text[j] in ';,':
                    return j + 1
                if text.startswith('else', j) or (j < len(text) and text[j] in '.?'):
                    i = j
                    continue
                # A struct pattern or literal goes on along its line
                # (`Kind { .. } if … =>`), and a match arm's pattern into its
                # guard on the next line; a block ends with its line.
                if j < len(text) and ('\n' not in text[i + 1:j] or arm_goes_on(text, j)):
                    i = j
                    continue
                return i + 1
        elif c == '<' and depth == 0 and i > 0 and (IDENT.match(text[i - 1]) or text[i - 1] == ':') and text[i + 1] not in '= ':
            angle += 1
        elif c == '>' and depth == 0 and angle > 0 and text[i - 1] not in '-=':
            angle -= 1
        elif c == '|' and depth == 0 and opens_closure(text, i):
            # A closure's parameters: their commas end nothing.
            i = text.index('|', i + 1) + 1
            continue
        elif c in ';,' and depth == 0 and angle == 0:
            return i + 1
        i += 1


def arm_goes_on(text, j):
    """Whether a match arm's pattern goes on at `j`: `=> …`, or a guard `if … =>`."""
    if text.startswith('=>', j):
        return True
    if not re.match(r'if\b', text[j:]):
        return False
    arrow = text.find('=>', j)
    brace = text.find('{', j)
    return arrow != -1 and (brace == -1 or arrow < brace)


def opens_closure(text, i):
    """Whether the `|` at `i` opens a closure's parameters (after `=`, `(`, `,` or `move`)."""
    if text.startswith('||', i):
        return False
    before = text[:i].rstrip()
    return before.endswith(('=', '(', ',')) and not before.endswith(('==', '!=', '<=', '>=')) or before.endswith('move')


def line_start(text, i):
    return text.rfind('\n', 0, i) + 1


def extend_up(text, start):
    """`start` moved up over the doc comments, comments and attributes right above its line."""
    s = line_start(text, start)
    if text[s:start].strip():
        return start
    while s > 0:
        prev = line_start(text, s - 1)
        line = text[prev:s - 1].strip()
        if line.startswith('//') and not line.startswith('//!') or (line.startswith('#[') and line.endswith(']')):
            s = prev
        else:
            break
    return s


def cut(text, start, end):
    """`text` without [start, end), and without the lines that leaves empty."""
    s = line_start(text, start)
    if not text[s:start].strip():
        start = s
    nl = text.find('\n', end)
    nl = len(text) if nl < 0 else nl
    if not text[end:nl].strip():
        end = min(nl + 1, len(text))
    text = text[:start] + text[end:]
    # No two empty lines in a row, none after an opening bracket or before
    # a closing one, as rustfmt leaves them.
    nl = text.find('\n', start)
    after = text[start:len(text) if nl < 0 else nl].strip()
    blank_before = text[max(start - 2, 0):start] == '\n\n'
    if blank_before and (not after or after[0] in ')]}'):
        text = text[:start - 1] + text[start:]
        start -= 1
    if not after and text[:start].rstrip(' \t\n').endswith(('{', '(', '[')) and nl >= 0:
        text = text[:start] + text[start:].lstrip(' \t').lstrip('\n')
    return text


def strip_rust(text, path):
    """`text` without the feature's items, or None for a file only the feature builds."""
    head = text.split('\n', 40)[:40]
    if any(line.strip() == CFG_FILE for line in head):
        return None
    while True:
        m = CFG_ON.search(text)
        if not m:
            break
        if text[line_start(text, m.start()):m.start()].lstrip().startswith('//'):
            raise ExportError('%s: the feature named in a comment: %r' % (path, text[m.start():m.end()]))
        start = extend_up(text, m.start())
        i = m.end()
        # Further attributes on the same item.
        while True:
            i = skip_trivia(text, i)
            if text.startswith('#[', i):
                i = text.index(']', i) + 1
            else:
                break
        text = cut(text, start, item_end(text, i))
    # The attribute alone, with its line when it stood on one.
    text = re.sub(r'(?m)^[ \t]*' + CFG_OFF.pattern + r'[ \t]*\n', '', text)
    text = CFG_OFF.sub('', text)
    return text


# --- JSON keys ------------------------------------------------------------


def json_keys():
    """(glob, dotted key path) pairs; `*` in a path is every element or key."""
    pairs = []
    for line in read_list('json-keys.txt'):
        glob, key = line.split()
        pairs.append((glob, key.split('.')))
    return pairs


def drop_key(node, parts):
    if len(parts) == 1:
        if isinstance(node, dict):
            node.pop(parts[0], None)
        return
    head, rest = parts[0], parts[1:]
    if head == '*':
        children = list(node.values()) if isinstance(node, dict) else node if isinstance(node, list) else []
    else:
        children = [node.get(head)] if isinstance(node, dict) else []
    for child in children:
        if child is not None:
            drop_key(child, rest)


def strip_json(text, keys):
    """`text` without the keys, written as the fixtures are: two spaces, UTF-8."""
    data = json.loads(text)
    for parts in keys:
        drop_key(data, parts)
    return json.dumps(data, indent=2, ensure_ascii=False) + '\n'


# --- the tree -------------------------------------------------------------


def is_text(data):
    if b'\0' in data[:8192]:
        return False
    try:
        data.decode('utf-8')
        return True
    except UnicodeDecodeError:
        return False


def build_tree(out, ref, worktree, public_url):
    private = read_list('private-paths.txt')
    keys = json_keys()
    if os.path.exists(out):
        if not os.path.isdir(out) or (os.listdir(out) and not os.path.exists(os.path.join(out, '.community-export'))):
            raise ExportError('%s is not empty and is not an earlier export: choose another folder' % out)
        shutil.rmtree(out)
    os.makedirs(out)
    kept = dropped = 0
    for path, data, executable in source_files(ref, worktree):
        if is_private(path, private):
            dropped += 1
            continue
        if is_text(data):
            text = data.decode('utf-8')
            text = strip_markers(text, path)
            if path.endswith('.rs'):
                text = strip_rust(text, path)
                if text is None:
                    dropped += 1
                    continue
            named = [k for g, k in keys if fnmatch.fnmatchcase(path, g)]
            if named:
                text = strip_json(text, named)
            data = text.replace(SOURCE_URL, public_url).encode('utf-8')
        write(out, path, data, executable)
        kept += 1
    overlay = os.path.join(HERE, 'overlay')
    for base, _, files in os.walk(overlay):
        for name in files:
            full = os.path.join(base, name)
            path = os.path.relpath(full, overlay)
            with open(full, 'rb') as f:
                data = f.read()
            if is_text(data):
                data = data.decode('utf-8').replace(SOURCE_URL, public_url).encode('utf-8')
            write(out, path, data, os.access(full, os.X_OK))
    with open(os.path.join(out, '.community-export'), 'w') as f:
        f.write('')
    return kept, dropped


def write(out, path, data, executable):
    full = os.path.join(out, path)
    os.makedirs(os.path.dirname(full), exist_ok=True)
    with open(full, 'wb') as f:
        f.write(data)
    if executable:
        os.chmod(full, 0o755)


# --- the leak check -------------------------------------------------------


def leak_check(out):
    patterns = [re.compile(p) for p in read_list('leak-patterns.txt')]
    found = []
    for base, dirs, files in os.walk(out):
        dirs[:] = [d for d in dirs if d not in ('node_modules', 'target', 'dist', '.git')]
        for name in files:
            full = os.path.join(base, name)
            path = os.path.relpath(full, out)
            if path == '.community-export':
                continue
            for p in patterns:
                if p.search(path):
                    found.append('%s: the path matches %s' % (path, p.pattern))
            with open(full, 'rb') as f:
                data = f.read()
            if not is_text(data):
                continue
            for n, line in enumerate(data.decode('utf-8').split('\n'), 1):
                for p in patterns:
                    m = p.search(line)
                    if m:
                        found.append('%s:%d: %s' % (path, n, m.group(0)))
                        break
    return found


# --- build, test, push ----------------------------------------------------

BUILD = [
    ['npm', 'ci'],
    ['npm', 'test'],
    ['npm', 'run', 'build'],
    # What the cut leaves behind is laid out by rustfmt, not by hand.
    ['cargo', 'fmt', '--all'],
    ['cargo', 'clippy', '--workspace', '--all-targets', '--locked', '--', '-D', 'warnings'],
    ['cargo', 'test', '--workspace', '--locked'],
]


def build_and_test(out):
    for cmd in BUILD:
        print('==> ' + ' '.join(cmd), flush=True)
        subprocess.run(cmd, cwd=out, check=True)


def push(out, public_url, ref):
    if not PUSH_ENABLED:
        raise ExportError('the push is turned off until firstmate confirms the token and the repository move '
                          '(community/README.md); the tree is in %s' % out)
    # One commit on the public repository's main per export, naming the
    # private commit it came from only by its date, never its hash or message.
    work = out + '.push'
    shutil.rmtree(work, ignore_errors=True)
    subprocess.run(['git', 'clone', '--depth', '1', public_url + '.git', work], check=True)
    for name in os.listdir(work):
        if name != '.git':
            p = os.path.join(work, name)
            shutil.rmtree(p) if os.path.isdir(p) and not os.path.islink(p) else os.remove(p)
    for name in os.listdir(out):
        if name in ('.community-export', 'node_modules', 'target', 'dist'):
            continue
        src = os.path.join(out, name)
        (shutil.copytree if os.path.isdir(src) else shutil.copy2)(src, os.path.join(work, name))
    subprocess.run(['git', 'add', '--all'], cwd=work, check=True)
    if subprocess.run(['git', 'diff', '--cached', '--quiet'], cwd=work).returncode == 0:
        print('Nothing changed since the last export.')
        return
    date = git('log', '-1', '--format=%cs', ref).decode().strip()
    subprocess.run(['git', 'commit', '-m', 'Sync from the main repository (%s)' % date], cwd=work, check=True)
    subprocess.run(['git', 'push', 'origin', 'HEAD:main'], cwd=work, check=True)


def main(argv):
    ap = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    ap.add_argument('--out', required=True, help='the folder the community tree is written to (empty, or an earlier export)')
    ap.add_argument('--ref', default='main', help='the commit to export (default main)')
    ap.add_argument('--worktree', action='store_true', help='export the files as they are now instead of --ref')
    ap.add_argument('--public-url', default=DEFAULT_URL, help='the public repository (default %s)' % DEFAULT_URL)
    ap.add_argument('--build', action='store_true', help='build and test the community tree')
    ap.add_argument('--push', action='store_true', help='push to --public-url (turned off: PUSH_ENABLED)')
    args = ap.parse_args(argv)
    out = os.path.abspath(args.out)
    if out == ROOT or out.startswith(ROOT + os.sep):
        raise ExportError('write the community tree outside this repository, not in %s' % out)
    if args.push and args.worktree:
        raise ExportError('--push exports a commit, never --worktree')
    if args.push and not PUSH_ENABLED:
        raise ExportError('the push is turned off until firstmate confirms the token and the repository move (community/README.md)')
    kept, dropped = build_tree(out, args.ref, args.worktree, args.public_url.rstrip('/'))
    source = 'the working tree' if args.worktree else args.ref
    print('Community tree of %s in %s: %d files, %d left out.' % (source, out, kept, dropped))
    leaks = leak_check(out)
    if leaks:
        print('\n'.join(leaks))
        print('Leak check: %d lines name the private part.' % len(leaks), file=sys.stderr)
        return 1
    print('Leak check: nothing names the private part.')
    if args.build:
        build_and_test(out)
    if args.push:
        push(out, args.public_url.rstrip('/'), args.ref)
    return 0


if __name__ == '__main__':
    try:
        sys.exit(main(sys.argv[1:]))
    except ExportError as e:
        print('export.py: ' + str(e), file=sys.stderr)
        sys.exit(2)
