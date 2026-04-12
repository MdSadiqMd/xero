"use client"

import { useState } from "react"
import { CadenceShell } from "@/components/cadence/shell"
import { ProjectRail } from "@/components/cadence/project-rail"
import { PhaseView } from "@/components/cadence/phase-view"
import { AgentRuntime } from "@/components/cadence/agent-runtime"
import { ExecutionView } from "@/components/cadence/execution-view"
import { type Project, type View, MOCK_PROJECTS } from "@/components/cadence/data"

export default function CadenceApp() {
  const [activeProject, setActiveProject] = useState<Project>(MOCK_PROJECTS[0])
  const [activeView, setActiveView] = useState<View>("phases")

  return (
    <CadenceShell activeView={activeView} onViewChange={setActiveView} projectName={activeProject.name}>
      <ProjectRail
        projects={MOCK_PROJECTS}
        activeProject={activeProject}
        onSelectProject={setActiveProject}
      />
      {activeView === "phases" && <PhaseView project={activeProject} />}
      {activeView === "agent" && <AgentRuntime project={activeProject} />}
      {activeView === "execution" && <ExecutionView project={activeProject} />}
    </CadenceShell>
  )
}
