from typing import Tuple, List
import grpc
import os.path
import subprocess
import vim

import sce_pb2 as proto
import sce_pb2_grpc as rpc

SCRIPT_PATH = os.path.dirname(os.path.realpath(__file__))
ENGINE_PATH = os.path.join(SCRIPT_PATH, "../sce/target/debug/sce")

#proc = subprocess.Popen([ENGINE_PATH])

channel = grpc.insecure_channel('localhost:1486')
stub = rpc.SCEStub(channel)


def delete_ranges(src: str, ranges: List[proto.Range], point: proto.Point) -> Tuple[str, proto.Point]:
    lines = src.split('\n')
    new_content = []
    new_point = point

    i = 0
    for rng in ranges:
        start = rng.start
        end = rng.end

        if i < start:
            new_content.extend(lines[i:start.line])
        prefix = lines[start.line][:start.col]
        if prefix.strip():
            new_content.append(prefix)
        suffix = lines[end.line][end.col:]
        if suffix.strip():
            new_content.append(suffix)
        i = end.line + 1

        if end.line < point.line:
            deleted_lines = end.line - start.line
            if (not prefix) and (not suffix):
                deleted_lines += 1
            new_point.line -= deleted_lines

    return '\n'.join(new_content), new_point


def do_slice_window(direction):
    row, col = vim.current.window.cursor
    point = proto.Point(
        line=row-1,  # row is 1-indexed, we expect 0-indexed
        col=col,
    )

    content = '\n'.join(vim.current.buffer)

    req = proto.SliceRequest(
        source=proto.Source(
            filename=vim.current.buffer.name,
            content=content,
            point=point,
        ),
        direction=direction, # xx
    )
    res = stub.Slice(req)

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

    vim.current.buffer[:], new_point = delete_ranges(content, res.to_remove, point)
    vim.current.window.cursor = [new_point.line + 1, new_point.col]


def do_slice_fold(direction):
    row, col = vim.current.window.cursor
    point = proto.Point(
        line=row-1,
        col=col,
    )

    content = '\n'.join(vim.current.buffer)

    req = proto.SliceRequest(
        source=proto.Source(
            filename=vim.current.buffer.name,
            content=content,
            point=point,
        ),
        direction=direction, # xx
    )
    res = stub.Slice(req)

    vim.command('setlocal foldmethod=manual')

    for rng in res.to_remove:
        vim.command(f'{rng.start.line+1},{rng.end.line+1}fold')


def inline():
    row, col = vim.current.window.cursor
    point = proto.Point(
        line=row-1,
        col=col,
    )

    content = '\n'.join(vim.current.buffer)
    syntax = vim.eval('&syntax')

    defs = vim.eval("CocAction('definitions')")
    if not defs:
        print("No definition found")
        return

    req = proto.InlineRequest(
        source=proto.Source(
            filename=vim.current.buffer.name,
            content=content,
            point=point,
        ),
        target_content=content,
        target_point=proto.Point(
            line=int(defs[0]['range']['start']['line']),
            col=int(defs[0]['range']['start']['character']),
        ),
    )
    res = stub.Inline(req)

    vim.command('vnew')
    vim.command('setlocal buftype=nofile')
    vim.command('setlocal bufhidden=delete')
    vim.command('setlocal noswapfile')
    vim.command('setlocal nobuflisted')
    vim.command('set syntax='+syntax)
    vim.current.buffer[:] = res.content.split('\n')
