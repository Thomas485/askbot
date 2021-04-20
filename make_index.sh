
case $1 in
  "optimized")
    cd web
    elm make src/Main.elm --optimize --output ../index.js
    echo '<!DOCTYPE HTML>
    <html> <head> <meta charset="UTF-8"> <title>Main</title> <script>' > ../index.html
    uglifyjs ../index.js --compress 'pure_funcs=[F2,F3,F4,F5,F6,F7,F8,F9,A2,A3,A4,A5,A6,A7,A8,A9],pure_getters,keep_fargs=false,unsafe_comps,unsafe' | uglifyjs --mangle >> ../index.html
    echo '</script></head> <body> <div id="myapp"></div> <script> var app = Elm.Main.init({ node: document.getElementById("myapp") }); </script> </body> </html>' >> ../index.html
    ;;
  "debug")
    cd web
    elm make src/Main.elm --debug --output ../index.html
    ;;
esac
