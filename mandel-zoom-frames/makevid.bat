echo on
cd /d "%~dp0"
echo y | ffmpeg.exe -framerate 60 -i frames/frame_%%04d.png -vf "scale=-2:-2" -c:v libx264 -pix_fmt yuv420p movie.mp4
pause