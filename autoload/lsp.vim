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

function! lsp#codeLensAction() abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  let l:params = s:Position()
  return rpc#call('codeLensAction', l:params)
endfunction

function! lsp#codeAction() abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  let l:params = extend(s:SelectionRange(), { 'text_document': expand('%:p') })
  return rpc#call('textDocument/codeAction', l:params)
endfunction

function! lsp#rename(new_name) abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  let l:params = extend(s:Position(), { 'language_id': &filetype, 'new_name': a:new_name })
  return rpc#call('textDocument/rename', l:params)
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

function! lsp#completion(callback) abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  return rpc#callWithCallback('textDocument/completion', s:Position(), a:callback)
endfunction

"{{{ PRIVATE FUNCTIONS
function! s:SelectionRange(...) abort
  let l:mode = mode()

  if l:mode ==# 'v'
    let [line_start, column_start] = getpos("'<")[1:2]
    let [line_end, column_end] = getpos("'>")[1:2]

    return { 'range': {
          \ 'start': { 'line': line_start, 'column': column_start },
          \ 'end': { 'line': line_end, 'column': column_end },
          \ }}
  endif

  let l:line = line('.')
  let l:col = col('.')
  return { 'range': {
        \ 'start': { 'line': l:line, 'column': l:col },
        \ 'end': { 'line': l:line, 'column': l:col },
        \ }}
endfunction

function! s:Position(...) abort
  let l:line = line('.')
  let l:col = col('.')
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
