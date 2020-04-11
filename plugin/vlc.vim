call call('vim#start', [])

nnoremap <silent>gl         :call vlc#codeLensAction()<CR>
nnoremap <silent>ga         :call vlc#codeAction()<CR>
vnoremap <silent>ga         :call vlc#codeAction()<CR>
nnoremap <silent>gd         :call vlc#goToDefinition()<CR>
nnoremap <silent>K          :call vlc#hover()<CR>
nnoremap <silent>R          :call vlc#rename()<CR>
nnoremap <silent>gr         :call vlc#references()<CR>
nnoremap <silent>gi         :call vlc#implementation()<CR>
nnoremap <c-s>              :call vlc#start()<CR>
nnoremap <c-k>              :call vlc#stop()<CR>

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
