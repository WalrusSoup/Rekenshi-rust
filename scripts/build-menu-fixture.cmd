@echo off
call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat" >nul
if errorlevel 1 exit /b 1
cl /nologo /LD /EHsc /O2 /MD /std:c++17 /DMYGUI_DONT_REPLACE_NULLPTR /Itarget\references\KenshiLib\Include\mygui tests\fixtures\native-menu.cpp /Fo:target\native-menu.obj /link /LIBPATH:target\references\KenshiLib\Libraries\mygui MyGUIEngine_x64.lib /OUT:target\native-menu.dll /IMPLIB:target\native-menu.lib
