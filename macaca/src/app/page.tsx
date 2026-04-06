import Link from 'next/link';

export default function Home() {
  return (
    <div className="min-h-screen flex flex-col items-center justify-center p-8 bg-gray-50">
      <div className="text-center">
        <h1 className="text-4xl font-bold text-gray-900 mb-4">React Button Component</h1>
        <p className="text-gray-600 mb-8">A reusable, customizable button component with TypeScript and Tailwind CSS</p>
        <Link href="/examples/button" className="px-6 py-3 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors">
          View Examples
        </Link>
      </div>
    </div>
  );
}