let s:responses = {}
let s:callbacks = {}
let s:id = 0
let s:job = -1

function! s:getID() abort
  let l:id = s:id
  let s:id = s:id + 1
  return l:id
endfunction

function! rpc#start(binpath, config) abort
    let cmd = [a:binpath]
    if a:config !=# v:null
      let cmd = add(cmd, '--config')
      let cmd = add(cmd, a:config)
      echom 'INFO: Using VLC config in: ' . a:config
    endif

  if has('nvim')
    let s:job = jobstart(cmd, {
        \ 'on_stdout': function('rpc#read'),
        \ 'on_stderr': function('vim#handleError'),
      \ })

    if s:job == 0
      echoerr 'invalid arguments'
      return 0
    elseif s:job == -1
      echoerr 'not executable'
      return 0
    else
      return 1
    endif
  else
    echoerr 'Not supported: only neovim is supported'
    return 0
  endif
endfunction

" TODO: needs to be separated in replySuccess and replyError
function! rpc#reply(id, params) abort
  call s:doSend('success', a:params, a:id)
endfunction

function! rpc#callWithCallback(method, params, callback) abort
  let l:id = rpc#call(a:method, a:params)
  let s:callbacks[l:id] = a:callback
endfunction

function! rpc#call(method, params) abort
  let l:id = s:getID()
  call s:doSend(a:method, a:params, l:id)
  return l:id
endfunction

function! rpc#notify(method, params) abort
  call s:doSend(a:method, a:params)
endfunction

function! s:doSend(method, params, ...) abort
  " do not send if binary hasn't been executed
  if s:job <= 0
    return 0
  endif

  let l:params = a:params
  if type(params) == type({})
      let l:params = extend({
            \ 'bufnr': bufnr(''),
            \ 'language_id': &filetype,
            \ }, l:params)
  endif

  let l:message = { 'jsonrpc': '2.0' }
  if a:method ==# 'success'
    let l:message['result'] = a:params
  elseif a:method ==# 'error'
    let l:message['error'] = a:params
  else
    let l:message['method'] = a:method
    let l:message['params'] = l:params
  endif

  if len(a:000) !=# 0
    let l:message['id'] = a:1
  endif

  let l:content = json_encode(l:message)
  let l:content_length = 'Content-Length: ' . len(l:content) . "\r\n\r\n"
  if !chansend(s:job, l:content_length)
    echoerr 'An error ocurred while communicating with the language client'
  endif

  if has('nvim')
    return !chansend(s:job, l:content)
  elseif has('channel')
    return ch_sendraw(s:job, l:content)
  else
    echoerr 'Not supported: not nvim nor vim with +channel.'
  endif
endfunction

let s:content_length = 0
let s:message = ''
function! rpc#read(job, lines, event) abort
  while len(a:lines) > 0
    let l:line = remove(a:lines, 0)
    if l:line ==# ''
      continue
    endif

    if s:content_length ==# 0
      let s:content_length = str2nr(substitute(l:line, '.*Content-Length:', '', ''))
      continue
    endif

    let s:message .= strpart(l:line, 0, s:content_length + 1)
    if s:content_length < strlen(l:line)
      call insert(a:lines, strpart(s:message, s:content_length + 1), 0)
      let s:content_length = 0
    else
      let s:content_length = s:content_length - strlen(l:line)
    endif

    if s:content_length > 0
      continue
    endif

    let s:message = trim(s:message)
    let l:message = json_decode(s:message)
    let s:message = ''

    if type(l:message) !=# type({})
      echoerr 'message type is not dict: ' . l:message
      continue
    endif

    if has_key(l:message, 'result') || has_key(l:message, 'error')
      let l:message_id = l:message['id']
      let Callback = s:callbacks[l:message_id]
      call Callback(l:message['result'])
      continue
    endif

    let l:method = l:message['method']
    let l:params = l:message['params']

    " TODO: replace this huge elseif block with just calling the vim function with the same name as
    " the jsonrpc method.
    " for example:
    "   { jsonrpc: 2.0, method: vim#showMmessage, params: [] }

    " shows a message in the bottom bar
    if l:method ==# 'showMessage'
      return vim#showMessage(l:params)
    " applies a list of edits
    elseif l:method ==# 'applyEdits'
      " l:params is a slice with the following format:
      "   {edits: [{lines: [{lnum: 2, text: "new text" }]}], text_document: "filename.go" }
      for l:changes in l:params
        call vim#applyChanges(l:changes)
      endfor
    " registers the completion engine with NCM2
    elseif l:method ==# 'registerNCM2Source'
      return vlc#registerNCM2Source(l:params)
    " opens and populates a floating window
    elseif l:method ==# 'showFloatingWindow'
      return vim#showFloatingWindow(l:params)
    " shows a list of items in fzf
    elseif l:method ==# 'showFZF'
      return vim#showFZF(l:params['items'], l:params['sink'])
    " shows the preview window and sets it's content
    elseif l:method ==# 'showPreview'
      return vim#showPreview(l:params)
    " shows the vitual texts for the current buffer
    elseif l:method ==# 'setVirtualTexts'
      return vim#setVirtualTexts(l:params)
    " shows the quickfix window and sets it's content
    elseif l:method ==# 'setQuickfix'
      return vim#setQuickfix(l:params)
    " sets the signs in the gutter
    elseif l:method ==# 'setSigns'
      return vim#setSigns(l:params)
    " evaluates a command and returns immediately
    elseif l:method ==# 'call'
      call vim#eval(l:params)
    " evaluates a command, waits for the response and replies to the server
    elseif l:method ==# 'eval'
      let l:res = vim#eval(l:params)
      if has_key(l:message, 'id')
        call rpc#reply(l:message['id'], l:res)
      endif
    elseif l:method ==# 'execute'
      let l:res = vim#execute(l:params)
      if has_key(l:message, 'id')
        call rpc#reply(l:message['id'], l:res)
      endif
    endif
  endwhile
endfunction
