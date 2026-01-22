import { useProjects } from '../../contexts/ProjectContext';

export function ProjectProgress() {
  const { currentProject } = useProjects();

  if (!currentProject) {
    return null;
  }

  // Calculate task statistics from the board
  const board = currentProject.board;
  const totalTasks = board.columns.reduce((sum, col) => sum + col.cards.length, 0);
  const completedTasks = board.columns
    .filter((col) => col.id === 'done')
    .reduce((sum, col) => sum + col.cards.length, 0);
  const percentComplete = totalTasks > 0 ? Math.round((completedTasks / totalTasks) * 100) : 0;

  return (
    <div className="project-progress">
      <div className="project-progress__header">
        <span className="project-progress__label">CURRENT PROJECT</span>
      </div>
      <div className="project-progress__name">{currentProject.name}</div>
      <div className="project-progress__stats">Tasks: {totalTasks} total</div>
      <div className="project-progress__bar">
        <div
          className="project-progress__fill"
          style={{ width: `${percentComplete}%` }}
        />
      </div>
      <div className="project-progress__percent">
        {percentComplete}% complete ({completedTasks}/{totalTasks} tasks done)
      </div>
    </div>
  );
}

export default ProjectProgress;
