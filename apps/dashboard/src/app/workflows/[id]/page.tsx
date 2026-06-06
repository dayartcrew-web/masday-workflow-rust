import WorkflowDetailPage from './workflow-detail-page'

export function generateStaticParams() {
  return [{ id: '_' }]
}

export default function Page() {
  return <WorkflowDetailPage />
}
