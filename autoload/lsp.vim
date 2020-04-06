let s:initialized = v:false

function! lsp#initialize() abort
  call rpc#call('initialize', {})
  let s:initialized = v:true
endfunction

function! lsp#didOpen() abort
  if s:initialized ==# v:false
    call lsp#initialize()
  endif

  call s:sendFileLifecycleEvent('textDocument/didOpen')
endfunction

function! lsp#didSave() abort
  call s:sendFileLifecycleEvent('textDocument/didSave')
endfunction

function! lsp#didChange() abort
  call s:sendFileLifecycleEvent('textDocument/didChange')
endfunction

function! lsp#didClose() abort
  call s:sendFileLifecycleEvent('textDocument/didClose')
endfunction

function! s:sendFileLifecycleEvent(event) abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  call rpc#notify(a:event, s:TextDocument())
  return 1
endfunction

function! lsp#exit() abort
  call rpc#notify('exit', v:null)
endfunction

function! lsp#shutdown() abort
  call rpc#call('shutdown', v:null)
endfunction

function! lsp#rename(new_name) abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  return rpc#call('textDocument/rename', {
        \ 'text_document_position': extend(s:Position(), {'language_id': &filetype}),
        \ 'new_name': a:new_name
        \ })
endfunction


function! lsp#hover() abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  return rpc#call('textDocument/hover', s:Position())
endfunction

function! lsp#implementation() abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  call rpc#call('textDocument/implementation', s:Position())
endfunction

function! lsp#references() abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  call rpc#call('textDocument/references', s:Position())
endfunction

function! lsp#goToDefinition() abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  call rpc#call('textDocument/definition', s:Position())
endfunction

function! lsp#completion(...) abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  return rpc#callAndWait('textDocument/completion', s:Position())
endfunction

"{{{ PRIVATE FUNCTIONS
function! s:Position(...) abort
  let l:line = line('.') - 1
  let l:col = col('.') - 1
  let l:path = expand('%:p')

  return {
        \ 'line': l:line,
        \ 'column': l:col,
        \ 'text_document': l:path,
        \}
endfunction

function! s:TextDocument(...) abort
  let l:text = s:text()
  return extend({
        \ 'text': l:text,
        \}, s:Position())
endfunction

function! s:text(...) abort
    let l:buf = get(a:000, 0, '')

    let l:lines = getbufline(l:buf, 1, '$')
    if len(l:lines) > 0 && l:lines[-1] !=# '' && &fixendofline
        let l:lines += ['']
    endif
    return join(l:lines, "\n")
endfunction
"}}}
