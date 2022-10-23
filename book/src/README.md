# mandelbrot

### Usage
mandelbrot FILE PIXELS UPPERLEFT LOWERRIGHT RENDERMETHOD  

RENDERMETHOD:  
* single: render without parallel library
* crossbeam: render with 8 threads by crossbeam
* rayon: render by rayon

### Example
cargo run mandel.png 1280x960 -2.0,1 0.6,-1 rayon

<img src="img/mandel.png">
