let s:initialized = v:false

function! lsp#initialize() abort
  call rpc#call('initialize', {})
  let s:initialized = v:true
endfunction

function! lsp#did_open() abort
  if !vlc#is_server_running(&filetype)
    call vlc#start()
  endif

  if s:initialized ==# v:false
    call lsp#initialize()
  endif

  call s:send_lifecycle_event('textDocument/didOpen')
endfunction

function! lsp#did_save() abort
  call s:send_lifecycle_event('textDocument/didSave')
endfunction

function! lsp#did_change() abort
  call s:send_lifecycle_event('textDocument/didChange')
endfunction

function! lsp#did_close() abort
  call s:send_lifecycle_event('textDocument/didClose')
endfunction

function! s:send_lifecycle_event(event) abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  call rpc#notify(a:event, s:text_document())
  return 1
endfunction

function! lsp#exit() abort
  call rpc#notify('exit', v:null)
endfunction

function! lsp#shutdown() abort
  call rpc#call('shutdown', v:null)
endfunction

function! lsp#code_lens_action() abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  let l:params = s:position()
  return rpc#call('vlc/codeLensAction', l:params)
endfunction

function! lsp#code_action() abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  let l:params = extend(s:selection_range(), { 'text_document': expand('%:p') })
  return rpc#call('textDocument/codeAction', l:params)
endfunction

function! lsp#formatting() abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  let l:params = {}
  return rpc#call('textDocument/formatting', l:params)
endfunction

function! lsp#rename(new_name) abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  let l:params = extend(s:position(), { 'language_id': &filetype, 'new_name': a:new_name })
  return rpc#call('textDocument/rename', l:params)
endfunction

function! lsp#hover() abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  return rpc#call('textDocument/hover', s:position())
endfunction

function! lsp#implementation() abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  call rpc#call('textDocument/implementation', s:position())
endfunction

function! lsp#references() abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  call rpc#call('textDocument/references', s:position())
endfunction

function! lsp#definition() abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  call rpc#call('textDocument/definition', s:position())
endfunction

function! lsp#completion(callback) abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  return rpc#call_with_callback('textDocument/completion', s:position(), a:callback)
endfunction

function! lsp#diagnostic_detail() abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  let l:params = s:position()
  return rpc#call('vlc/diagnosticDetail', l:params)
endfunction

" TODO: not sure what to do with the result of completionItem/resolve
function! lsp#completion_item_resolve(callback) abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  let l:user_data = get(v:completed_item, 'user_data', '')
  if l:user_data ==# ''
    return
  endif

  let l:params = {
        \ 'position': s:position(),
        \ 'completion_item': v:completed_item,
        \ }
  return rpc#call_with_callback('completionItem/resolve', l:params, a:callback)
endfunction

"{{{ PRIVATE FUNCTIONS
function! s:selection_range(...) abort
  let l:mode = mode()

  if l:mode ==# 'v'
    let [line_start, column_start] = getpos("'<")[1:2]
    let [line_end, column_end] = getpos("'>")[1:2]

    return { 'range': {
          \ 'start': { 'line': line_start - 1, 'column': column_start },
          \ 'end': { 'line': line_end - 1, 'column': column_end },
          \ }}
  endif

  let l:line = line('.')
  let l:col = col('.')
  return { 'range': {
        \ 'start': { 'line': l:line - 1, 'column': l:col },
        \ 'end': { 'line': l:line - 1, 'column': l:col },
        \ }}
endfunction

function! s:position(...) abort
  let l:line = line('.')
  let l:col = col('.')
  let l:path = expand('%:p')

  return {
        \ 'line': l:line,
        \ 'column': l:col,
        \ 'text_document': l:path,
        \}
endfunction

function! s:text_document(...) abort
  let l:text = s:text()
  return extend({
        \ 'text': l:text,
        \}, s:position())
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
