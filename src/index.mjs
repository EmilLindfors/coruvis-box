import { h, render } from "https://unpkg.com/preact?module";
import htm from "https://unpkg.com/htm?module";

const html = htm.bind(h);

function App(props) {
  return html`
    <div class="grid">
      <div class="grid-row-lg">
        <h2>${props.data.inf.host}</h2>
        <h3>${props.data.inf.os}</h3>
      </div>
      <div class="grid-row">${JSON.stringify(props.data.pss)}</div>
      <div class="grid-row">
        ${props.data.cpu.map((cpu) => {
          return html`<div class="bar">
            <div class="bar-inner" style="width: ${cpu}%"></div>
            <label>${cpu.toFixed(2)}%</label>
          </div>`;
        })}
      </div>
      <div class="grid-row">
        <div class="bar">
          <div
            class="bar-inner"
            style="width: ${(props.data.mem.used / props.data.mem.total) *
            100}%"
          ></div>
          <label
            >${(props.data.mem.used / 1000000).toFixed(1)}/
            ${(props.data.mem.total / 1000000).toFixed(1)} Mb</label
          >
        </div>
      </div>
      <div class="grid-row-lg">
        <table>
          <thead>
            <tr>
              <th>Process Name</th>
              <th>Memory Usage</th>
              <th>CPU usage</th>
              <th>Read / Written to disk</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            <tr class="first-row">
              <td>${props.data.prc.this.name}</td>
              <td>${(props.data.prc.this.mem / 1000000).toFixed(1)} Mb</td>
              <td>${props.data.prc.this.cpu.toFixed(2)}%</td>
              <td>
                ${(props.data.prc.this.readBytes / 1000000).toFixed(1)} /
                ${(props.data.prc.this.writtenBytes / 1000000).toFixed(1)} Mb
              </td>
              <td>${props.data.prc.this.status}</td>
            </tr>
            ${props.data.prc.others.map((pr) => {
              return html`<tr>
                <td>${pr.name}</td>
                <td>${(pr.mem / 1000000).toFixed(1)} Mb</td>
                <td>${pr.cpu.toFixed(2)}%</td>

                <td>
                  ${(pr.readBytes / 1000000).toFixed(1)} /
                  ${(pr.writtenBytes / 1000000).toFixed(1)}
                </td>
                <td>${pr.status}</td>
              </tr>`;
            })}
          </tbody>
        </table>
      </div>
    </div>
  `;
}

let i = 0;

// let update = async () => {
//   let response = await fetch("/api/cpus");
//   if (response.status !== 200) {
//     throw new Error(`HTTP error! status: ${response.status}`);
//   }

//   let json = await response.json();
//   render(html`<${App} cpus=${json}></${App}>`, document.body);
// };

// update();
// setInterval(update, 200);

let url = new URL("/realtime/cpus", window.location.href);
// http => ws
// https => wss
url.protocol = url.protocol.replace("http", "ws");

let ws = new WebSocket(url.href);
ws.onmessage = (ev) => {
  let json = JSON.parse(ev.data);
  render(html`<${App} data=${json}></${App}>`, document.body);
};
