document.addEventListener("keydown", function(e) {
    if (e.keyCode == 39) {
        goForwards();
    } else if (e.keyCode == 37) {
        goBackwards();
    }
}, false);
