call call('vim#start', [])

nnoremap <silent>gd   :call vlc#goToDefinition()<CR>
nnoremap <silent>K    :call vlc#hover()<CR>
nnoremap <silent>gr   :call vlc#references()<CR>
nnoremap <silent>gi   :call vlc#implementation()<CR>
nnoremap <c-s>        :call vlc#start()<CR>

command! VLCDefintion     call vlc#goToDefinition()
command! VLCStop          call vlc#shutdown()
command! VLCStart         call vim#start()


call sign_define('vlc_error', {
  \ 'text' : '!!',
  \ 'texthl' : 'Error',
  \ 'linehl' : 'Search'})

call sign_define('vlc_warn', {
  \ 'text' : '--',
  \ 'texthl' : 'Warn',
  \ 'linehl' : 'Search'})

augroup vlc
    autocmd!
    autocmd FileType      *   call vlc#didOpen()
    autocmd TextChanged   *   call lsp#didChange()
    autocmd BufWritePost  *   call lsp#didSave()
    autocmd BufWinLeave   *   call lsp#didClose()
    autocmd VimLeavePre   *   call lsp#exit()
    autocmd TextChangedP  *   call lsp#didChange()
    autocmd TextChangedI  *   call lsp#didChange()
    " autocmd FileReadPost * call vlc#didOpen()
    " autocmd FileReadPost * call vlc#didOpen()
    " autocmd BufReadPost * call vlc#didOpen()
    " autocmd BufNewFile * call vlc#didOpen()
    " autocmd TextChanged * call vlc#didOpen()
    " autocmd BufDelete * call LanguageClient#handleBufDelete()
    " autocmd TextChanged * call LanguageClient#handleTextChanged()
    " autocmd TextChangedI * call LanguageClient#handleTextChanged()
    " if exists('##TextChangedP')
    "     autocmd TextChangedP * call LanguageClient#handleTextChanged()
    " endif
    " autocmd CursorMoved * call LanguageClient#handleCursorMoved()
    " autocmd VimLeavePre * call LanguageClient#handleVimLeavePre()

    " autocmd CompleteDone * call LanguageClient#handleCompleteDone()

    " if get(g:, 'LanguageClient_signatureHelpOnCompleteDone', 0)
    "     autocmd CompleteDone *
    "                 \ call LanguageClient#textDocument_signatureHelp({}, 's:HandleOutputNothing')
    " endif
augroup END
