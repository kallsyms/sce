" Prevent loading the plugin multiple times
if exists('g:sce_loaded')
    finish
endif
let g:sce_loaded = 1

if !has("python3")
    echo "vim has to be compiled with +python3 to run this"
    finish
endif

let s:plugin_root_dir = fnamemodify(resolve(expand('<sfile>:p')), ':h')

python3 << EOF
import sys
import vim
plugin_root_dir = vim.eval('s:plugin_root_dir')
sys.path.insert(0, plugin_root_dir)
import sce
EOF

function! s:slice_window(direction)
    python3 sce.do_slice_window(vim.eval("a:direction"))
endfunction

function! s:slice_fold(direction)
    python3 sce.do_slice_fold(vim.eval("a:direction"))
endfunction

command! -nargs=0 SliceBackwardW call s:slice_window('Backward')
command! -nargs=0 SliceForwardW call s:slice_window('Forward')

command! -nargs=0 SliceBackwardF call s:slice_fold('Backward')
command! -nargs=0 SliceForwardF call s:slice_fold('Forward')
