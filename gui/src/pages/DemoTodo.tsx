import { useState } from 'react';

interface Todo {
  id: number;
  text: string;
  done: boolean;
}

export default function DemoTodo() {
  const [todos, setTodos] = useState<Todo[]>([]);
  const [text, setText] = useState('');

  const add = () => {
    const trimmed = text.trim();
    if (!trimmed) return;
    setTodos([...todos, { id: Date.now(), text: trimmed, done: false }]);
    setText('');
  };

  const toggle = (id: number) => {
    setTodos(todos.map((t) => (t.id === id ? { ...t, done: !t.done } : t)));
  };

  const remove = (id: number) => {
    setTodos(todos.filter((t) => t.id !== id));
  };

  const clearCompleted = () => {
    setTodos(todos.filter((t) => !t.done));
  };

  return (
    <div className="p-6 space-y-4" data-testid="demo-todo">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Demo: Todo List</h1>
          <p className="text-sm text-gray-400">Used for Playwright smoke checks.</p>
        </div>
      </div>

      <div className="flex gap-3">
        <input
          data-testid="todo-input"
          className="flex-1 px-3 py-2 rounded-md bg-background-surface border border-background-mid"
          placeholder="Add a task..."
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && add()}
        />
        <button
          data-testid="todo-add"
          className="px-4 py-2 rounded-md bg-accent-teal text-background-deep font-semibold"
          onClick={add}
        >
          Add
        </button>
      </div>

      <div className="rounded-md border border-background-mid divide-y divide-background-mid">
        {todos.length === 0 ? (
          <div className="p-4 text-gray-500 text-sm">No todos yet.</div>
        ) : (
          todos.map((todo) => (
            <div key={todo.id} className="p-3 flex items-center gap-3">
              <input
                type="checkbox"
                data-testid={`todo-checkbox-${todo.id}`}
                checked={todo.done}
                onChange={() => toggle(todo.id)}
              />
              <span className={`flex-1 ${todo.done ? 'line-through text-gray-500' : ''}`}>{todo.text}</span>
              <button
                data-testid={`todo-delete-${todo.id}`}
                className="text-red-400 hover:text-red-300 text-sm"
                onClick={() => remove(todo.id)}
              >
                Delete
              </button>
            </div>
          ))
        )}
      </div>

      <div className="flex items-center justify-between text-sm text-gray-400">
        <span data-testid="todo-counter">
          {todos.length} item{todos.length === 1 ? '' : 's'}
        </span>
        <button
          data-testid="todo-clear-completed"
          className="text-accent-teal hover:underline"
          onClick={clearCompleted}
        >
          Clear completed
        </button>
      </div>
    </div>
  );
}
