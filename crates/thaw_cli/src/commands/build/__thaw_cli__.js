window.onload = function () {
    const socket = new WebSocket(
        `ws://${window.location.host}/__thaw_cli_ws__`
    );

    socket.addEventListener("message", function (event) {
        if (event.data == "RefreshPage") {
            window.location.reload();
        }
    });
};
