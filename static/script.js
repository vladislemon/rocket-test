const eventSource = new EventSource("/message");

eventSource.addEventListener("message", (event) => {
    let newElement = document.createElement("li");
    let eventList = document.getElementById("list");

    let message = JSON.parse(event.data);
    console.log(message);
    newElement.textContent = `${message.time}: ${message.text}`;
    eventList.appendChild(newElement);
});
