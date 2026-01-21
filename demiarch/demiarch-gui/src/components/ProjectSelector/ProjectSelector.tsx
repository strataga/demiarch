import { useState, useRef, useEffect } from 'react';
import { useProjects } from '../../contexts/ProjectContext';
import './ProjectSelector.css';

export function ProjectSelector() {
  const { projects, currentProject, selectProject } = useProjects();
  const [isOpen, setIsOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  // Close dropdown when clicking outside
  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    }

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  // Close dropdown on Escape key
  useEffect(() => {
    function handleEscape(event: KeyboardEvent) {
      if (event.key === 'Escape') {
        setIsOpen(false);
      }
    }

    if (isOpen) {
      document.addEventListener('keydown', handleEscape);
      return () => document.removeEventListener('keydown', handleEscape);
    }
  }, [isOpen]);

  const handleSelectProject = (projectId: string) => {
    selectProject(projectId);
    setIsOpen(false);
  };

  if (projects.length === 0) {
    return (
      <div className="project-selector project-selector--empty">
        <span className="project-selector__no-projects">No projects</span>
      </div>
    );
  }

  return (
    <div className="project-selector" ref={dropdownRef}>
      <button
        className="project-selector__trigger"
        onClick={() => setIsOpen(!isOpen)}
        aria-haspopup="listbox"
        aria-expanded={isOpen}
      >
        {currentProject ? (
          <>
            <span
              className="project-selector__color-dot"
              style={{ backgroundColor: currentProject.color }}
            />
            <span className="project-selector__name">{currentProject.name}</span>
          </>
        ) : (
          <span className="project-selector__placeholder">Select a project</span>
        )}
        <span className={`project-selector__chevron ${isOpen ? 'project-selector__chevron--open' : ''}`}>
          &#9662;
        </span>
      </button>

      {isOpen && (
        <div className="project-selector__dropdown" role="listbox">
          {projects.map((project) => (
            <button
              key={project.id}
              className={`project-selector__option ${
                project.id === currentProject?.id ? 'project-selector__option--selected' : ''
              }`}
              onClick={() => handleSelectProject(project.id)}
              role="option"
              aria-selected={project.id === currentProject?.id}
            >
              <span
                className="project-selector__color-dot"
                style={{ backgroundColor: project.color }}
              />
              <div className="project-selector__option-content">
                <span className="project-selector__option-name">{project.name}</span>
                {project.description && (
                  <span className="project-selector__option-description">{project.description}</span>
                )}
              </div>
              {project.id === currentProject?.id && (
                <span className="project-selector__check">&#10003;</span>
              )}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

export default ProjectSelector;
