const input = document.getElementById("new-todo");
const addBtn = document.getElementById("add-btn");
const list = document.getElementById("todo-list");
const clearBtn = document.getElementById("clear-completed");
const counter = document.getElementById("counter");

const todos = [];

function render() {
  list.innerHTML = "";
  todos.forEach((todo, idx) => {
    const li = document.createElement("li");
    li.className = todo.done ? "done" : "";
    li.dataset.testid = `todo-${idx}`;

    const checkbox = document.createElement("input");
    checkbox.type = "checkbox";
    checkbox.checked = todo.done;
    checkbox.addEventListener("change", () => toggle(idx));

    const text = document.createElement("span");
    text.className = "text";
    text.textContent = todo.text;

    const destroy = document.createElement("button");
    destroy.className = "destroy";
    destroy.textContent = "✕";
    destroy.addEventListener("click", () => remove(idx));

    li.append(checkbox, text, destroy);
    list.appendChild(li);
  });

  counter.textContent = `${todos.length} item${todos.length === 1 ? "" : "s"}`;
}

function add() {
  const text = input.value.trim();
  if (!text) return;
  todos.push({ text, done: false });
  input.value = "";
  render();
}

function toggle(idx) {
  todos[idx].done = !todos[idx].done;
  render();
}

function remove(idx) {
  todos.splice(idx, 1);
  render();
}

function clearCompleted() {
  for (let i = todos.length - 1; i >= 0; i--) {
    if (todos[i].done) {
      todos.splice(i, 1);
    }
  }
  render();
}

addBtn.addEventListener("click", add);
input.addEventListener("keydown", (e) => {
  if (e.key === "Enter") add();
});
clearBtn.addEventListener("click", clearCompleted);

render();
