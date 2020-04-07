let s:running = v:false

function! vim#start() abort
  if s:running ==# v:true
    return 0
  endif

  let l:binpath = '/home/martin/dev/vlc/target/debug/vlc'

  if executable(l:binpath) != 1
    echoerr 'binary ' . l:binpath . ' not found'
    return 0
  endif

  call rpc#start(l:binpath)
  let s:running = v:true
endfunction

function! vim#handleError(job, lines, event) abort
    echoerr json_decode(a:lines)
endfunction

function! vim#execute(commands) abort
  let l:res = []
  for l:params in a:commands
    let l:action = l:params['action']
    let l:command = l:params['command']
    if l:action ==# 'execute'
      execute l:command
      let l:res = add(l:res, v:null)
    elseif l:action ==# 'call'
      let l:result = eval(l:command)
      let l:res = add(l:res, l:result)
    endif
  endfor

  return l:res
endfunction

function! vim#eval(params) abort
  let l:res = eval(a:params['command'])
  return l:res
endfunction

function! vim#applyChanges(changes) abort
  execute 'edit' a:changes['text_document']
  for change in a:changes['edits']
    for line in change['lines']
      let l:lnum = line['lnum']
      call setline(l:lnum, line['text'])
    endfor
  endfor
  execute ':w'
endfunction

function! vim#setVirtualTexts(params) abort
  if type(a:params) !=# type([])
    echoerr 'virtual texts list is not a list'
  endif

  let l:prefix = ''
  if !exists('*nvim_buf_set_virtual_text')
      return
  endif

  call nvim_buf_clear_namespace(0, -1, 0, -1)

  for vt in a:params
    call nvim_buf_set_virtual_text(0, -1, vt['line'], [[l:prefix . vt['text'], vt['hl_group']]], {})
  endfor
endfunction

function! vim#setQuickfix(params) abort
  if type(a:params) !=# type([])
    echoerr 'quickfix list is not a list'
  endif

  let l:params = []
  for l:line in a:params
    let l:params = add(l:params, l:line)
  endfor

  call setqflist(l:params)
endfunction

function! vim#showMessage(params) abort
  let l:level = 'INFO'
  if a:params['level'] == 1
    let l:level = 'ERROR'
  elseif a:params['level'] == 2
    let l:level = 'WARNING'
  elseif a:params['level'] == 3
    let l:level = 'INFO'
  elseif a:params['level'] == 4
    let l:level = 'LOG'
  endif

  echo l:level . ': ' . a:params['message']
endfunction

function! vim#setSigns(params) abort
  for l:sign in a:params
    if bufexists(l:sign['file'])
      call sign_place(l:sign['id'], '', 'vlc_warn', l:sign['file'], { 'lnum': l:sign['line'] })
    endif
  endfor
endfunction

function! vim#showPreview(params)
  let l:filetype = a:params['filetype']
  let l:lines = split(a:params['text'], "\n")

  let l:name = 'vim-lc'
  let l:command = "silent! pedit! +setlocal\\ " .
    \ "buftype=nofile\\ nobuflisted\\ " .
    \ "noswapfile\\ nonumber\\ " .
    \ 'filetype=' . l:filetype . ' ' . l:name
  exe l:command

  if has('nvim')
      call nvim_buf_set_lines(bufnr(l:name), 0, -1, 0, l:lines)
  else
      call setbufline(l:name, 1, l:lines)
  endif
endfunction
