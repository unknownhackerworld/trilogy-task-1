@echo off
REM Speech Translator — Dev launcher
REM Sets CUDA 12.6 and LLVM paths before starting Tauri dev mode

set CUDA_PATH=C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.6
set CudaToolkitDir=C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.6\
set LIBCLANG_PATH=C:\Program Files\LLVM\bin

REM Prepend CUDA 12.6 bin so it wins over 11.8 in system PATH
set PATH=C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.6\bin;%PATH%
set PATH=C:\Program Files\LLVM\bin;%PATH%
set PATH=%USERPROFILE%\.cargo\bin;%PATH%

echo CUDA: %CUDA_PATH%
echo nvcc:
nvcc --version 2>nul | findstr /C:"release"

npx tauri dev
