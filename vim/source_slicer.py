import json
import os.path
import subprocess
import vim

SCRIPT_PATH = os.path.dirname(os.path.realpath(__file__))

def do_slice():
    row, col = vim.current.window.cursor
    # row is 1-indexed, we expect 0-indexed
    row -= 1

    cmd = json.dumps({
        'filename': vim.current.buffer.name,
        'content': '\n'.join(vim.current.buffer),
        'point': [row, col],
    }).encode('utf-8')

    res = subprocess.run([os.path.join(SCRIPT_PATH, "../slicer/target/debug/slicer")], input=cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT).stdout.decode('utf-8')
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

    vim.current.buffer[:] = res.split('\n')
    # TODO: get equivalent cursor position
    #vim.current.window.cursor = (row + 1, col)
