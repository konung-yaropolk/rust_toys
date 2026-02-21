echo on
cd /d "%~dp0"
echo y | ffmpeg.exe -framerate 30 -i frames/frame_%%04d.png -c:v libx264 -pix_fmt yuv420p zoom.mp4
pause