let s:running = {}

function! vlc#did_open() abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  if !has_key(s:running, &filetype)
    call vlc#start()
  endif

  call lsp#did_open()
endfunction

function! vlc#formatting() abort
  call lsp#formatting()
endfunction

function! vlc#rename() abort
  let l:new_name = input('Enter new name: ')
  call lsp#rename(l:new_name)
endfunction

function! vlc#code_lens_action() abort
  call lsp#code_lens_action()
endfunction

function! vlc#code_action() abort
  call lsp#code_action()
endfunction

function! vlc#implementation() abort
  call lsp#implementation()
endfunction

function! vlc#references() abort
  call lsp#references()
endfunction

function! vlc#hover() abort
  call lsp#hover()
endfunction

function! vlc#definition() abort
  call lsp#definition()
endfunction

function! vlc#exit() abort
  call lsp#exit()
endfunction

function! vlc#shutdown() abort
  call lsp#shutdown()
endfunction

function! vlc#resolve_completion() abort
  echom expand('<cword>')
  " call lsp#completionItemResolve(funcref('s:doEcho'))
endfunction

function! vlc#start() abort
  call rpc#call('start', {'language_id': &filetype})
  let s:running[&filetype] = v:true
endfunction

function! vlc#stop() abort
  call rpc#call('shutdown', {'language_id': &filetype})
  let s:running[&filetype] = v:false
endfunction

" omnifunc completion func
function! vlc#completion(findstart, base) abort
  if a:findstart ==# 1
    return col('.')
  endif

  call lsp#completion(funcref('vlc#do_complete'))
endfunction

" ncm2 completion callback to populate completion list
function! vlc#do_complete(res) abort
  call complete(col('.'), a:res['words'])
endfunction
