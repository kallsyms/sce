from typing import Tuple, List
import json
import os.path
import subprocess
import vim

SCRIPT_PATH = os.path.dirname(os.path.realpath(__file__))

def delete_ranges(src: str, ranges: List[Tuple[int, int]], point: Tuple[int, int]) -> Tuple[str, Tuple[int, int]]:
    lines = src.split('\n')
    new_content = []
    new_point = list(point)

    i = 0
    for start, end in ranges:
        if i < start:
            new_content.extend(lines[i:start[0]])
        prefix = lines[start[0]][:start[1]]
        if prefix.strip():
            new_content.append(prefix)
        suffix = lines[end[0]][end[1]:]
        if suffix.strip():
            new_content.append(suffix)
        i = end[0] + 1

        if end[0] < point[0]:
            deleted_lines = end[0] - start[0]
            if (not prefix) and (not suffix):
                deleted_lines += 1
            new_point[0] -= deleted_lines

    return '\n'.join(new_content), tuple(new_point)


def do_slice_window(direction):
    row, col = vim.current.window.cursor
    # row is 1-indexed, we expect 0-indexed
    row -= 1

    content = '\n'.join(vim.current.buffer)
    point = [row, col]

    cmd = json.dumps({
        'filename': vim.current.buffer.name,
        'content': content,
        'point': point,
        'direction': direction,
    }).encode('utf-8')

    res = json.loads(subprocess.run([os.path.join(SCRIPT_PATH, "../slicer/target/debug/slicer")], input=cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT).stdout.decode('utf-8'))

    # grab the syntax for the current file
    syntax = vim.eval('&syntax')

    vim.command('vnew')
    # https://github.com/preservim/tagbar/blob/0243b19920a683df531f19bb7fb80c0ff83927dd/autoload/tagbar.vim#L989
    vim.command('setlocal buftype=nofile')
    vim.command('setlocal bufhidden=delete')
    vim.command('setlocal noswapfile')
    vim.command('setlocal nobuflisted')

    # TODO: make this persist when moving around windows
    vim.command('set statusline=Slice')

    vim.command('set syntax='+syntax)

    vim.current.buffer[:], new_point = delete_ranges(content, res['ranges_to_remove'], point)
    new_point[0] += 1  # 1-based line number
    vim.current.window.cursor = new_point

def do_slice_fold(direction):
    row, col = vim.current.window.cursor
    # row is 1-indexed, we expect 0-indexed
    row -= 1

    content = '\n'.join(vim.current.buffer)
    point = [row, col]

    cmd = json.dumps({
        'filename': vim.current.buffer.name,
        'content': content,
        'point': point,
        'direction': direction,
    }).encode('utf-8')

    res = json.loads(subprocess.run([os.path.join(SCRIPT_PATH, "../slicer/target/debug/slicer")], input=cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT).stdout.decode('utf-8'))
    print(res)

    vim.command('setlocal foldmethod=manual')

    for start, end in res['ranges_to_remove']:
        vim.command(f'{start[0]+1},{end[0]+1}fold')