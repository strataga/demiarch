import { Routes, Route } from 'react-router-dom';
import Layout from './components/Layout';
import Dashboard from './pages/Dashboard';
import Projects from './pages/Projects';
import ProjectDetail from './pages/ProjectDetail';
import Kanban from './pages/Kanban';
import Agents from './pages/Agents';
import Settings from './pages/Settings';
import ConflictResolution from './pages/ConflictResolution';
import ToastContainer from './components/ToastContainer';

function App() {
  return (
    <>
      <Routes>
        <Route path="/" element={<Layout />}>
          <Route index element={<Dashboard />} />
          <Route path="projects" element={<Projects />} />
          <Route path="projects/:projectId" element={<ProjectDetail />} />
          <Route path="projects/:projectId/kanban" element={<Kanban />} />
          <Route path="projects/:projectId/conflicts" element={<ConflictResolution />} />
          <Route path="projects/:projectId/conflicts/:conflictId" element={<ConflictResolution />} />
          <Route path="agents" element={<Agents />} />
          <Route path="settings" element={<Settings />} />
        </Route>
      </Routes>
      <ToastContainer />
    </>
  );
}

export default App;
