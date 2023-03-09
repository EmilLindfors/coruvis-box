import { h, render } from "https://unpkg.com/preact?module";
import htm from "https://unpkg.com/htm?module";

const html = htm.bind(h);

function App(props) {
  return html`
    <div class="grid">
      <div class="grid-row-lg float">
        <h2><span class="darker">Host name:</span> ${
          props.data.general.host
        }</h2>
        <h2><span class="darker">Os:</span> ${props.data.general.os}</h3>
        <h2>
          <span class="darker">CPU:</span> ${props.data.general.cpu.cores} Core
          ${props.data.general.cpu.name} @ ${props.data.general.cpu.mhz} MHz
        </h2>
      </div>

      <div class="grid-row">
      <h3>CPU virtual core usage</h3>
        ${props.data.cpu.map((cpu) => {
          return html`<div class="bar">
            <div class="bar-inner" style="width: ${cpu}%"></div>
            <label>${cpu.toFixed(2)}%</label>
          </div>`;
        })}
      </div>
      <div class="grid-row">
       <h3>Memory usage</h3>
       ${JSON.stringify(props.data.mem)}
        <div class="bar">
          <div
            class="bar-inner"
            style="width: ${
              (props.data.mem.used / props.data.mem.total) * 100
            }%"
          ></div>
          <label
            >${props.data.mem.used} /
            ${props.data.mem.total} Mb</label
          >
        </div>
      </div>
        <div class="grid-row">
       <h3>Disk usage</h3>
        <div class="bar">
          <div
            class="bar-inner"
            style="width: ${
              (props.data.hdd.used / props.data.hdd.total) * 100
            }%"
          ></div>
          <label
            >${props.data.hdd.used}/
            ${props.data.hdd.total} Mb</label
          >
        </div>
      </div>
      <div class="grid-row-lg">
        <h3>Top 10 processes sorted by memory usage</h3>
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
            ${props.data.prc.map((pr) => {
              return html`<tr class=${pr[0][1] && "coruvis"}>
                <td>${pr[1][1]}</td>
                <td>${parseFloat(pr[2][1])} Mb</td>
                <td>${parseFloat(pr[3][1]).toFixed(2)}%</td>
                <td>${pr[4][1]} / ${pr[5][1]}</td>
                <td>${pr[6][1]}</td>
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
