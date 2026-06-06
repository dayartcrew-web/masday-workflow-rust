import MemoryDetailPage from './memory-detail-page'

export function generateStaticParams() {
  return [{ id: '_' }]
}

export default function Page() {
  return <MemoryDetailPage />
}
