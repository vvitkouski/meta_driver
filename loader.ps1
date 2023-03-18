.\gload.exe gdrv.sys meta_driver.sys
.\gload.exe meta_driver.sys

E:
cd .\Projects\meta_driver
cd .\target\x86_64-pc-windows-msvc\debug



rustc -Zunpretty=expanded src\main.rs



.\gload.exe gdrv.sys kernelmode.sys
.\gload.exe kernelmode.sys




Mouse class object pointer: FFFFC70199FC4DC0
Hid object pointer: FFFFC7019A06AE10


[norsefire]: CLASS FFFFC70199FC4DC0
[norsefire]: HID FFFFC7019A06AE10

.\gload.exe meta_driver.sys /l meta_driver.sys