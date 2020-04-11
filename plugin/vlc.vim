call call('vim#start', [])

set omnifunc=vlc#completion

command! VLCDefintion       call vlc#goToDefinition()
command! VLCImplementation  call vlc#implementation()
command! VLCReferences      call vlc#references()
command! VLCCodeAction      call vlc#codeAction()
command! VLCCodeLensAction  call vlc#codeLensAction()
command! VLCRename          call vlc#rename()
command! VLCHover           call vlc#hover()
command! VLCStop            call vlc#shutdown()
command! VLCStart           call vim#start()

call sign_define('vlc_error', {
  \ 'text' : '!!',
  \ 'texthl' : 'Error' })

call sign_define('vlc_warn', {
  \ 'text' : '--',
  \ 'texthl' : 'Warn'})

augroup vlc
    autocmd!
    autocmd FileType      *   call vlc#didOpen()
    autocmd TextChanged   *   call lsp#didChange()
    autocmd BufWritePost  *   call lsp#didSave()
    autocmd BufWinLeave   *   call lsp#didClose()
    autocmd VimLeavePre   *   call lsp#exit()
    autocmd TextChangedP  *   call lsp#didChange()
    autocmd TextChangedI  *   call lsp#didChange()
augroup END
