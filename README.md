# interpol

This is a small rust program for interpolating between two or more images
and saving the output frames to be made into a video / for some other use.

## Running
The program can be run like this:
`cargo run --release -- <image1> <image2> [<imageN>]`

Where the images are in interpolation order: `im1 -> im2 -> im3`...

You can then use `ffmpeg` to stitch the frames together:

- if you have a reasonably modern nvidia card this should be faster:
    `ffmpeg -f image2 -r 30 -i 'frames/frame_%09d.png' -c:v h264_nvenc -qp 0 -y video.mp4`

- otherwise this should work just fine:
    `ffmpeg -f image2 -r 30 -i 'frames/frame_%09d.png' -qp 0 -y video.mp4`

## Examples
An example output can be seen here: [https://imgur.com/a/WO1KBaF](https://imgur.com/a/WO1KBaF).
