let s:running = {}

function! vlc#didOpen() abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  if !has_key(s:running, &filetype)
    call vlc#start()
  endif

  call lsp#didOpen()
endfunction

function! vlc#completion(findstart, base) abort
  if a:findstart ==# 1
    return col('.')
  endif

  let l:response = lsp#completion(a:000)
  return l:response['result']
endfunction

function! vlc#rename() abort
  let l:new_name = input('Enter new name: ')
  call lsp#rename(l:new_name)
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

function! vlc#goToDefinition() abort
  call lsp#goToDefinition()
endfunction

function! vlc#exit() abort
  call lsp#exit()
endfunction

function! vlc#shutdown() abort
  call lsp#shutdown()
endfunction

function! vlc#start() abort
  call rpc#call('start', {'language_id': &filetype})
  let s:running[&filetype] = v:true
endfunction

