var PORT = 8080; // must match the messenger_port in settings.toml
var socket = new WebSocket("ws://localhost:" + PORT);
var o = new MutationObserver(function (mutations) {
    Array.from(mutations).forEach(function (m) {
        if(m.addedNodes.length && m.addedNodes[0].tagName.toLowerCase() === "div") {
            if(m.addedNodes.length>1) console.warn("Missing nodes! ", m);
            var d = m.addedNodes[0].querySelector("div[data-tooltip-content]:not([role=button])");
            if(d) {
                var messageText;
                var img = d.querySelector("div[role=presentation] > div > img.img");
                if(img) {
                    messageText = img.src;
                } else {
                    messageText = d.children.length >= 2 && d.querySelector("div[aria-label").getAttribute("aria-label");
                }
                var blockquote = d.querySelector("a > blockquote");
                if(blockquote) {
                    messageText = "> " + blockquote.querySelector("div > span").textContent + "\n" + messageText;
                }
                var messageAuthor;
                try {
                    messageAuthor = d.parentNode.parentNode.parentNode.children[0].children[0].children[0].children[0].children[0].getAttribute("alt");
                } catch(e) {
                    try {
                        messageAuthor = d.parentNode.parentNode.parentNode.parentNode.children[0].children[0].children[0].children[0].children[0].getAttribute("alt");
                    } catch(e) {
                        // don't care
                    }
                }
                if(messageText === "View message reactions") {
                    return;
                }
                if(messageText && messageAuthor) {
                    console.log(messageAuthor + ": " + messageText);
                    socket.send(messageAuthor + ": " + messageText);
                }
            }
        }
    });
})
o.observe(document.getElementsByClassName("uiScrollableAreaContent")[2], { childList: true, subtree: true })
