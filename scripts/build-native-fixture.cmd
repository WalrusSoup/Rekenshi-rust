@echo off
call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat" >nul
if errorlevel 1 exit /b 1
cl /nologo /c /EHsc /O2 /MD /FIcstdint /FIcstring /Itarget\references\RE_Kenshi\thirdparty target\references\RE_Kenshi\SIC.cpp /Fo:target\sic-reference.obj
if errorlevel 1 exit /b 1
cl /nologo /LD /EHsc /O2 /MD tests\fixtures\native-unwind.cpp target\sic-reference.obj /Fo:target\native-unwind.obj /link /LIBPATH:target\references\RE_Kenshi /OUT:target\native-unwind.dll /IMPLIB:target\native-unwind.lib
