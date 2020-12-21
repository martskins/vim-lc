let s:responses = {}
let s:callbacks = {}
let s:id = 0
let s:job = -1

function! s:get_id() abort
  let l:id = s:id
  let s:id = s:id + 1
  return l:id
endfunction

function! vlc#rpc#start(binpath, config) abort
    let cmd = [a:binpath]
    if a:config !=# v:null
      let cmd = add(cmd, '--config')
      let cmd = add(cmd, a:config)
      echom 'INFO: Using VLC config in: ' . a:config
    endif

  if has('nvim')
    let s:job = jobstart(cmd, {
        \ 'on_stdout': function('vlc#rpc#read'),
        \ 'on_stderr': function('vlc#handle_error'),
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
  elseif has('job')
    let s:job = job_start([a:binpath], {
       \ 'out_cb': function('vlc#rpc#read'),
       \ 'err_cb': function('vlc#handle_error'),
       \ })
    if job_status(s:job) !=# 'run'
       call s:Echoerr('LanguageClient: job failed to start or died!')
       return 0
    else
        return 1
    endif
  else
    echoerr 'Not supported: only neovim is supported'
    return 0
  endif
endfunction

function! vlc#rpc#read_vim(job, data) abort
    return vlc#rpc#read(a:job, [a:data], 'stdout')
endfunction

function! vlc#rpc#hande_error_vim(job, data) abort
    return vlc#rpc#handle_error(a:job, [a:data], 'stderr')
endfunction

" TODO: needs to be separated in replySuccess and replyError
function! vlc#rpc#reply(id, params) abort
  call s:do_send('success', a:params, a:id)
endfunction

function! vlc#rpc#call_with_callback(method, params, callback) abort
  let l:id = vlc#rpc#call(a:method, a:params)
  let s:callbacks[l:id] = a:callback
endfunction

function! vlc#rpc#call(method, params) abort
  let l:id = s:get_id()
  call s:do_send(a:method, a:params, l:id)
  return l:id
endfunction

function! vlc#rpc#notify(method, params) abort
  call s:do_send(a:method, a:params)
endfunction

function! s:do_send(method, params, ...) abort
  if !vlc#has_server_configured(&filetype)
    return
  endif

  " do not send if binary hasn't been executed
  if s:job <= 0
    return 0
  endif

  let l:params = a:params
  if type(params) == type({})
      let l:params = extend({
            \ 'bufnr': bufnr(''),
            \ 'filename': expand('%:p'),
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
function! vlc#rpc#read(job, lines, event) abort
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

    let l:message_id = v:null
    if has_key(l:message, 'id')
      let l:message_id = l:message['id']
    endif

    if has_key(l:message, 'result') || has_key(l:message, 'error')
      let Callback = s:callbacks[l:message_id]
      call Callback(l:message['result'])
      continue
    endif

    let l:method = l:message['method']
    let l:params = l:message['params']
    if l:method ==# 'vlc#show_fzf'
      return vlc#show_fzf(l:params['items'], l:params['sink'])
    " shows the vitual texts for the current buffer
    elseif l:method ==# 'call'
      call vlc#eval(l:params)
    " evaluates a command, waits for the response and replies to the server
    elseif l:method ==# 'eval'
      let l:res = vlc#eval(l:params)
      if has_key(l:message, 'id')
        call vlc#rpc#reply(l:message_id, l:res)
      endif
    elseif l:method ==# 'execute'
      let l:res = vlc#execute(l:params)
      if has_key(l:message, 'id')
        call vlc#rpc#reply(l:message_id, l:res)
      endif
    else
      let l:result = call(l:method, l:params)
      if l:message_id isnot v:null
        call vlc#rpc#reply(l:message_id, l:result)
      endif
    endif
  endwhile
endfunction
