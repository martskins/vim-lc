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

function! vlc#rename() abort
  let l:new_name = input('Enter new name: ')
  call lsp#rename(l:new_name)
endfunction

function! vlc#codeLensAction() abort
  call lsp#codeLensAction()
endfunction

function! vlc#codeAction() abort
  call lsp#codeAction()
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

function! vlc#stop() abort
  call rpc#call('shutdown', {'language_id': &filetype})
  let s:running[&filetype] = v:false
endfunction

function! vlc#registerNCM2Source(params) abort
  let l:complete_pattern = a:params['complete_pattern']
  let l:cpp = []
  for cp in l:complete_pattern
    let l:cpp = add(l:cpp, escape(cp, '.\/:'))
  endfor
  echom json_encode(l:cpp)
  call ncm2#register_source({
      \ 'name' : 'vlc',
      \ 'scope': [a:params['language_id']],
      \ 'priority': 9,
      \ 'mark': 'VLC',
      \ 'subscope_enable': 1,
      \ 'complete_length': -1,
      \ 'complete_pattern': l:cpp,
      \ 'on_complete': ['vlc#ncm2Completion'],
      \ })
endfunction

" ncm2 completion func
function! vlc#ncm2Completion(ctx) abort
  call lsp#completion(funcref('vlc#ncmDoComplete', [a:ctx]))
endfunction

" omnifunc completion func
function! vlc#completion(findstart, base) abort
  if a:findstart ==# 1
    return col('.')
  endif

  call lsp#completion(funcref('vlc#doComplete'))
endfunction

" ncm2 completion callback to populate completion list
function! vlc#doComplete(res) abort
  call complete(col('.'), a:res['words'])
endfunction

" ncm2 completion callback to populate completion list
function! vlc#ncmDoComplete(ctx, res) abort
  call ncm2#complete(a:ctx, col('.'), a:res['words'], 0)
endfunction
