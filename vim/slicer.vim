" Prevent loading the plugin multiple times
if exists('g:source_slicer_loaded')
    finish
endif
let g:source_slicer_loaded = 1

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
import source_slicer
EOF

function! s:slice()
    python3 source_slicer.do_slice()
endfunction

command! -nargs=0 Slice call s:slice()
