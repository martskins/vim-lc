function! vlc#lsp#initialize() abort
  call vlc#rpc#call('initialize', {})
endfunction

function! vlc#lsp#did_open() abort
  if !vlc#is_server_running(&filetype)
    call vlc#start()
    call vlc#lsp#initialize()
  endif

  call s:send_lifecycle_event('textDocument/didOpen')
endfunction

function! vlc#lsp#did_save() abort
  call s:send_lifecycle_event('textDocument/didSave')
endfunction

function! vlc#lsp#did_change() abort
  call s:send_lifecycle_event('textDocument/didChange')
endfunction

function! vlc#lsp#did_close() abort
  call s:send_lifecycle_event('textDocument/didClose')
endfunction

function! s:send_lifecycle_event(event) abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  call vlc#rpc#notify(a:event, s:text_document())
  return 1
endfunction

function! vlc#lsp#exit() abort
  call vlc#rpc#notify('exit', v:null)
endfunction

function! vlc#lsp#shutdown() abort
  call vlc#rpc#call('shutdown', v:null)
endfunction

function! vlc#lsp#code_lens_action() abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  let l:params = s:position()
  return vlc#rpc#call('vlc/codeLensAction', l:params)
endfunction

function! vlc#lsp#code_action() abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  let l:params = extend(s:selection_range(), { 'text_document': expand('%:p') })
  return vlc#rpc#call('textDocument/codeAction', l:params)
endfunction

function! vlc#lsp#formatting() abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  let l:params = {}
  return vlc#rpc#call('textDocument/formatting', l:params)
endfunction

function! vlc#lsp#rename(new_name) abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  let l:params = extend(s:position(), { 'language_id': &filetype, 'new_name': a:new_name })
  return vlc#rpc#call('textDocument/rename', l:params)
endfunction

function! vlc#lsp#hover() abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  return vlc#rpc#call('textDocument/hover', s:position())
endfunction

function! vlc#lsp#implementation() abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  call vlc#rpc#call('textDocument/implementation', s:position())
endfunction

function! vlc#lsp#references() abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  call vlc#rpc#call('textDocument/references', s:position())
endfunction

function! vlc#lsp#definition() abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  call vlc#rpc#call('textDocument/definition', s:position())
endfunction

function! vlc#lsp#completion(callback) abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  return vlc#rpc#call_with_callback('textDocument/completion', s:position(), a:callback)
endfunction

function! vlc#lsp#diagnostic_detail() abort
  if &buftype !=# '' || &filetype ==# '' || expand('%') ==# ''
    return 0
  endif

  let l:params = s:position()
  return vlc#rpc#call('vlc/diagnosticDetail', l:params)
endfunction

" TODO: not sure what to do with the result of completionItem/resolve
function! vlc#lsp#completion_item_resolve(callback) abort
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
  return vlc#rpc#call_with_callback('completionItem/resolve', l:params, a:callback)
endfunction

"{{{ PRIVATE FUNCTIONS
function! s:selection_range(...) abort
  let l:mode = mode()

  if l:mode ==# 'v'
    let [line_start, column_start] = getpos("'<")[1:2]
    let [line_end, column_end] = getpos("'>")[1:2]

    return { 'range': {
          \ 'start': { 'line': line_start - 1, 'column': column_start - 1},
          \ 'end': { 'line': line_end - 1, 'column': column_end - 1},
          \ }}
  endif

  let l:line = line('.')
  let l:col = col('.')
  return { 'range': {
        \ 'start': { 'line': l:line - 1, 'column': l:col - 1},
        \ 'end': { 'line': l:line - 1, 'column': l:col - 1},
        \ }}
endfunction

function! s:position(...) abort
  let l:line = line('.')
  let l:col = col('.')
  let l:path = expand('%:p')

  return {
        \ 'line': l:line,
        \ 'column': l:col,
        \ 'filename': l:path,
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
