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
  for change in a:changes['changes']
    let l:start = change['start']['line'] + 1
    let l:end = change['end']['line'] + 1
    execute l:start . ',' . l:end . 'd'
    let l:pos = getcurpos()
    let l:cursorline = l:pos[1] - 1
    let l:cursorcol = l:pos[2]
    call cursor(l:cursorline, l:cursorcol)
    echom change['lines'][0]
    call append(line('.'), change['lines'])
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
  let l:lines = a:params['lines']

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

function! vim#showFloatingWindow(params)
  let l:lines = a:params['lines']
  let width = min([&columns - 4, max([80, &columns - 20])])
  let height = len(l:lines) + 1
  let top = ((&lines - height) / 2) - 1
  let left = (&columns - width) / 2
  let opts = {
        \ 'relative': 'editor',
        \ 'row': top,
        \ 'col': left,
        \ 'width': width,
        \ 'height': height,
        \ 'style': 'minimal'
        \}

  set winhl=NormalNC:Floating
  let l:textbuf = nvim_create_buf(v:false, v:true)
  call nvim_open_win(l:textbuf, v:true, opts)
  setlocal filetype=markdown
  call append(0, l:lines)
endfunction

function! vim#showFZF(items, sink)
  call fzf#run(fzf#wrap({ 'source': a:items, 'sink': function(a:sink)}))
endfunction

" selection is a string with the following pattern:
"   {filename}:{line_number} \t{preview}
function! s:fzfLocationSink(selection) abort
  let l:parts = split(a:selection, ':')
  let l:filename = l:parts[0]
  let l:line = split(l:parts[1], '\t')[0]

  execute 'edit' l:filename
  call cursor(l:line, 0)
endfunction

function! s:resolveAction(method, selection) abort
  let l:line = line('.')
  let l:col = col('.')
  let l:path = expand('%:p')
  let l:params = {
        \'selection': a:selection,
        \'text_document': l:path,
        \'line': l:line,
        \'column': l:col
        \}
  call rpc#call(a:method, l:params)
endfunction

function! s:resolveCodeLensAction(selection) abort
  call s:resolveAction('resolveCodeLensAction', a:selection)
endfunction


function! s:resolveCodeAction(selection) abort
  call s:resolveAction('resolveCodeAction', a:selection)
endfunction

