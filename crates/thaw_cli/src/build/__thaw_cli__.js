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

socket.addEventListener("message", async (event) => {
    handleMessage(JSON.parse(event.data));
});

socket.addEventListener("close", async () => {
    handleMessage({
        type: "Custom",
        event: "thaw-cli:ws:disconnect",
        data: {
            webSocket: socket,
        },
    });
});

async function handleMessage(payload) {
    switch (payload.type) {
        case "Connected":
            console.debug(`[thaw-cli] connected.`);
            break;
        case "RefreshPage":
            pageReload();
            break;
        case "Custom":
            if (payload.event === "thaw-cli:ws:disconnect") {
                const socket = payload.data.webSocket;
                const url = new URL(socket.url);
                url.search = "";
                await waitForSuccessfulPing(url.href);
                location.reload();
            }
    }
}

async function waitForSuccessfulPing(socketUrl, ms = 1e3) {
    async function ping() {
        const socket = new WebSocket(socketUrl, "thaw-cli-ping");
        return new Promise((resolve) => {
            function onOpen() {
                resolve(true);
                close();
            }
            function onError() {
                resolve(false);
                close();
            }
            function close() {
                socket.removeEventListener("open", onOpen);
                socket.removeEventListener("error", onError);
                socket.close();
            }
            socket.addEventListener("open", onOpen);
            socket.addEventListener("error", onError);
        });
    }
    while (true) {
        if (document.visibilityState === "visible") {
            if (await ping()) {
                break;
            }
            await wait(ms);
        } else {
            await waitForWindowShow();
        }
    }
}
function wait(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}
function waitForWindowShow() {
    return new Promise((resolve) => {
        const onChange = async () => {
            if (document.visibilityState === "visible") {
                resolve();
                document.removeEventListener("visibilitychange", onChange);
            }
        };
        document.addEventListener("visibilitychange", onChange);
    });
}
