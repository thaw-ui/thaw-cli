function debounceReload(time) {
    let timer;
    return () => {
        if (timer) {
            clearTimeout(timer);
            timer = null;
        }
        timer = setTimeout(() => {
            window.location.reload();
        }, time);
    };
}
const pageReload = debounceReload(50);

console.debug("[thaw-cli] connecting...");
const socket = new WebSocket(`ws://${window.location.host}/__thaw_cli__`);

socket.addEventListener("message", function (event) {
    handleMessage(event.data);
});

function handleMessage(payload) {
    switch (payload.type) {
        case "Connected":
            console.debug(`[thaw-cli] connected.`);
            break;
        case "RefreshPage":
            pageReload();
            break;
    }
}
