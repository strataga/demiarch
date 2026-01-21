import { createContext, useContext, useState, useCallback, type ReactNode } from 'react';
import type { Project } from '../types/project';
import { createProject as createProjectHelper, PROJECT_COLORS } from '../types/project';
import type { KanbanBoard } from '../components/KanbanBoard/types';

interface ProjectContextValue {
  projects: Project[];
  currentProject: Project | null;
  selectProject: (projectId: string) => void;
  createProject: (name: string, board: KanbanBoard, description?: string, color?: string) => Project;
  updateProject: (projectId: string, updates: Partial<Omit<Project, 'id' | 'createdAt'>>) => void;
  deleteProject: (projectId: string) => void;
  updateProjectBoard: (board: KanbanBoard) => void;
}

const ProjectContext = createContext<ProjectContextValue | null>(null);

interface ProjectProviderProps {
  children: ReactNode;
  initialProjects?: Project[];
}

export function ProjectProvider({ children, initialProjects = [] }: ProjectProviderProps) {
  const [projects, setProjects] = useState<Project[]>(initialProjects);
  const [currentProjectId, setCurrentProjectId] = useState<string | null>(
    initialProjects.length > 0 ? initialProjects[0].id : null
  );

  const currentProject = projects.find((p) => p.id === currentProjectId) ?? null;

  const selectProject = useCallback((projectId: string) => {
    setCurrentProjectId(projectId);
  }, []);

  const createProjectFn = useCallback(
    (name: string, board: KanbanBoard, description?: string, color?: string): Project => {
      const newProject = createProjectHelper(
        name,
        board,
        description,
        color ?? PROJECT_COLORS[projects.length % PROJECT_COLORS.length]
      );
      setProjects((prev) => [...prev, newProject]);
      // Auto-select if this is the first project
      if (projects.length === 0) {
        setCurrentProjectId(newProject.id);
      }
      return newProject;
    },
    [projects.length]
  );

  const updateProject = useCallback(
    (projectId: string, updates: Partial<Omit<Project, 'id' | 'createdAt'>>) => {
      setProjects((prev) =>
        prev.map((project) =>
          project.id === projectId
            ? { ...project, ...updates, updatedAt: new Date() }
            : project
        )
      );
    },
    []
  );

  const deleteProject = useCallback(
    (projectId: string) => {
      setProjects((prev) => {
        const filtered = prev.filter((p) => p.id !== projectId);
        // If we deleted the current project, select another one
        if (currentProjectId === projectId && filtered.length > 0) {
          setCurrentProjectId(filtered[0].id);
        } else if (filtered.length === 0) {
          setCurrentProjectId(null);
        }
        return filtered;
      });
    },
    [currentProjectId]
  );

  const updateProjectBoard = useCallback(
    (board: KanbanBoard) => {
      if (!currentProjectId) return;
      updateProject(currentProjectId, { board });
    },
    [currentProjectId, updateProject]
  );

  return (
    <ProjectContext.Provider
      value={{
        projects,
        currentProject,
        selectProject,
        createProject: createProjectFn,
        updateProject,
        deleteProject,
        updateProjectBoard,
      }}
    >
      {children}
    </ProjectContext.Provider>
  );
}

export function useProjects() {
  const context = useContext(ProjectContext);
  if (!context) {
    throw new Error('useProjects must be used within a ProjectProvider');
  }
  return context;
}
