let s:running = {}
let s:started = v:false

function! vlc#formatting() abort
  call vlc#lsp#formatting()
endfunction

function! vlc#rename() abort
  let l:new_name = input('Enter new name: ')
  call vlc#lsp#rename(l:new_name)
endfunction

function! vlc#code_lens_action() abort
  call vlc#lsp#code_lens_action()
endfunction

function! vlc#code_action() abort
  call vlc#lsp#code_action()
endfunction

function! vlc#implementation() abort
  call vlc#lsp#implementation()
endfunction

function! vlc#references() abort
  call vlc#lsp#references()
endfunction

function! vlc#hover() abort
  call vlc#lsp#hover()
endfunction

function! vlc#definition() abort
  call vlc#lsp#definition()
endfunction

function! vlc#exit() abort
  call vlc#lsp#exit()
endfunction

function! vlc#shutdown() abort
  call vlc#lsp#shutdown()
endfunction

function! vlc#resolve_completion() abort
  echom expand('<cword>')
  " call vlc#lsp#completionItemResolve(funcref('s:doEcho'))
endfunction

function! vlc#diagnostic_detail() abort
  call vlc#lsp#diagnostic_detail()
endfunction

function! vlc#start() abort
  if &filetype ==# ''
    return
  endif

  call vlc#rpc#call('start', {'language_id': &filetype})
  let s:running[&filetype] = v:true
endfunction

function! vlc#stop() abort
  call vlc#rpc#call('shutdown', {'language_id': &filetype})
  let s:running[&filetype] = v:false
endfunction

" omnifunc completion func
function! vlc#completion(findstart, base) abort
  if a:findstart ==# 1
    return col('.')
  endif

  call vlc#lsp#completion(funcref('vlc#do_complete'))
endfunction

" ncm2 completion callback to populate completion list
function! vlc#do_complete(res) abort
  call complete(col('.'), a:res['words'])
endfunction

function! vlc#is_server_running(filetype)
  return get(s:running, a:filetype, v:false)
endfunction

function! vlc#has_server_configured(filetype)
  let servers = get(g:, 'vlc#servers', {})
  return has_key(servers, a:filetype)
endfunction

function! vlc#run() abort
  if s:started ==# v:true
    return 0
  endif

  let l:binpath = expand('~/dev/vim-lc/target/debug/vlc')
  " let l:binpath = expand('~/dev/vim-lc/target/release/vlc')
  " let l:binpath = expand('~/.vim/plugged/vim-lc/target/release/vlc')
  if exists('g:vlc#binpath')
    let l:binpath = expand(g:vlc#binpath)
  endif

  let l:config = v:null
  if exists('g:vlc#config')
    let l:config = expand(g:vlc#config)
  endif

  if executable(l:binpath) != 1
    echoerr 'binary ' . l:binpath . ' not found'
    return 0
  endif

  call vlc#rpc#start(l:binpath, l:config)
  let s:started = v:true
endfunction

function! vlc#handle_error(job, lines, event) abort
    echoerr json_decode(a:lines)
endfunction

function! vlc#execute(commands) abort
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

function! vlc#eval(params) abort
  let l:res = eval(a:params['command'])
  return l:res
endfunction

function! vlc#apply_edits(edits) abort
  for l:edit in a:edits
    call vlc#apply_edit(l:edit)
  endfor
endfunction

function! vlc#apply_edit(changes) abort
  try
    execute 'edit' a:changes['text_document']
  catch
  endtry

  for change in a:changes['changes']
    let l:start_line = change['start']['line']
    let l:start_col = change['start']['column']

    let l:end_line = change['end']['line']
    let l:end_col = change['end']['column'] - 2

    let l:first_line = getline(l:start_line)
    if len(l:first_line) ==# l:start_col - 1
      let l:start_line += 1
      let l:start_col = 0
    endif

    " if change['end'] !=# change['start']
      let l:command = 'normal! ' . l:start_line . 'G0' . l:start_col
      if l:start_col > 0
        let l:command .= 'l'
      endif
      let l:command .= 'v'.  l:end_line . 'G0' . l:end_col
      if l:end_col > 0
        let l:command .= 'l'
      endif
      let l:command .= 'c'

      echom l:command
      call execute(l:command)
      execute 'normal! lha' . join(change['lines'], '\n')
    " else
    "   call cursor(l:start_line, l:start_col)
    "   execute 'normal! lha' . join(change['lines'], '\n')
    " endif
    " call append(line('.'), change['lines'])
    " normal! c

      " echom l:start_line
      " echom l:end_line
      " echom json_encode(change['lines'])
    " if l:start_line < l:end_line
      " " if the change happens in multiple lines
      " " delete the first line from start_col to end
      " " normal! D
      " " delete all lines in between first and last
      " " not sure why 2, but it works
      " " execute l:start_line + 1 . ',' . l:end_line . 'd'
      " " delete all characters from the start of the last line to end_col
      " let cnum = l:start_col
      " while cnum < l:end_col
      "   norma! x
      "   let cnum += 1
      " endwhile
    " elseif l:start_line == l:end_line
      " " if the change happens in a single line
      " " delete all characters between start_col and end_col
      " let cnum = l:start_col
      " while cnum < l:end_col
      "   norma! x
      "   let cnum += 1
      " endwhile
    " endif

    " if l:start_line < l:end_line
    "   " if the change happens in multiple lines
    "   " append all lines after the cursor
    "   call append(line('.'), change['lines'])
    " else
    "   " if change happens in a single line
    "   " insert the change after the cursor
    "   execute 'normal! ha' . change['lines'][0]
    " endif
  endfor
  execute ':w'
endfunction

function! vlc#set_virtual_texts(params) abort
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

function! vlc#set_quickfix(params) abort
  if type(a:params) !=# type([])
    echoerr 'quickfix list is not a list'
  endif

  let l:params = []
  for l:line in a:params
    let l:params = add(l:params, l:line)
  endfor

  call setqflist(l:params)
endfunction

function! vlc#show_message(params) abort
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

function! vlc#clear_signs(file) abort
  if !exists('*sign_unplace')
    execute 'sign unplace * group=VLC buffer=' . a:file
  else
    call sign_unplace('VLC', { 'buffer': a:file })
  endif
endfunction

function! vlc#place_sign(id, name, file, line) abort
  if !exists('*sign_place')
    execute 'sign place id=' . a:id . ' name=' . a:name . ' file=' . a:file . ' line=' . a:line
  endif

  call sign_place(a:id, 'VLC', a:name, a:file, { 'lnum': a:line })
endfunction

function! vlc#set_signs(file, params) abort
  call vlc#clear_signs(a:file)
  for l:sign in a:params
    call vlc#place_sign(l:sign['id'], l:sign['sign'], a:file, l:sign['line'])
  endfor
endfunction

function! vlc#show_preview(params)
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

function! vlc#show_float_win(params)
  let l:lines = []
  for line in a:params['lines']
    let l:lines = add(l:lines, ' ' . line . ' ')
  endfor

  let width = 82
  let height = len(l:lines) + 3
  let top = -height
  let left = 0
  let opts = {
        \ 'relative': 'cursor',
        \ 'row': top,
        \ 'col': left,
        \ 'width': width,
        \ 'height': height
        \}

  let l:pos = getcurpos()
  let l:textbuf = nvim_create_buf(v:false, v:true)
  let win_handle = nvim_open_win(l:textbuf, v:true, opts)
  call append(1, l:lines)
  setlocal filetype=markdown
  setlocal buftype=nofile nobuflisted bufhidden=wipe nonumber norelativenumber signcolumn=no modifiable
  setlocal nomodified nomodifiable
  setlocal wrap
  normal! gg
  wincmd p

  augroup vlc-hover
    execute 'autocmd CursorMoved,CursorMovedI,InsertEnter <buffer> call s:close_floating_win('. win_handle . ', ' . string(l:pos) . ')'
  augroup END
endfunction

function! s:close_floating_win(win_handle, pos) abort
  " we do not wish to close the window is moving from inside it back to the original buffer
  if a:pos ==# getcurpos()
    return
  endif

  call nvim_win_close(a:win_handle, 1)
  autocmd! vlc-hover
endfunction

function! vlc#show_fzf(items, sink)
  call fzf#run(fzf#wrap({ 'source': a:items, 'sink': function(a:sink)}))
endfunction

function! vlc#show_locations(items, sink) abort
  call setloclist(0, a:items)
  :lopen
endfunction

function! vlc#selection(items, sink) abort
  let l:options = map(copy(a:items), { key, val -> printf('%d) %s', key + 1, val ) })
  call inputsave()
  let l:selection = inputlist(l:options)
  call inputrestore()

  if !l:selection || l:selection > len(l:options)
      return
  endif

  call call(a:sink, [l:selection])
endfunction

" selection is a string with the following pattern:
"   {filename}:{line_number} \t{preview}
function! s:location_sink(selection) abort
  let l:parts = split(a:selection, ':')
  let l:filename = l:parts[0]
  let l:line = split(l:parts[1], '\t')[0]

  execute 'edit' l:filename
  call cursor(l:line, 0)
endfunction

function! s:resolve_action(method, selection) abort
  let l:line = line('.')
  let l:col = col('.')
  let l:path = expand('%:p')
  let l:params = {
        \'selection': a:selection - 1,
        \'text_document': l:path,
        \'line': l:line,
        \'column': l:col
        \}
  call vlc#rpc#call(a:method, l:params)
endfunction

function! s:resolve_code_lens_action(selection) abort
  call s:resolve_action('vlc/resolveCodeLensAction', a:selection)
endfunction


function! s:resolve_code_action(selection) abort
  call s:resolve_action('vlc/resolveCodeAction', a:selection)
endfunction

function! vlc#trigger_completion()
  call feedkeys("\<C-x>\<C-o>", "n")
endfunction
