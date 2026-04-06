import React from 'react';
import Button from '@/components/ui/Button';

const ButtonExamples = () => {
  return (
    <div className="min-h-screen p-8 bg-gray-50">
      <div className="max-w-4xl mx-auto">
        <h1 className="text-3xl font-bold text-gray-900 mb-8">Button Component Examples</h1>
        
        {/* Variants */}
        <div className="mb-8">
          <h2 className="text-xl font-semibold text-gray-800 mb-4">Variants</h2>
          <div className="flex flex-wrap gap-4">
            <Button variant="primary">Primary</Button>
            <Button variant="secondary">Secondary</Button>
            <Button variant="outline">Outline</Button>
            <Button variant="ghost">Ghost</Button>
            <Button variant="link">Link</Button>
          </div>
        </div>

        {/* Sizes */}
        <div className="mb-8">
          <h2 className="text-xl font-semibold text-gray-800 mb-4">Sizes</h2>
          <div className="flex flex-wrap gap-4 items-center">
            <Button size="small">Small</Button>
            <Button size="medium">Medium</Button>
            <Button size="large">Large</Button>
          </div>
        </div>

        {/* Loading States */}
        <div className="mb-8">
          <h2 className="text-xl font-semibold text-gray-800 mb-4">Loading States</h2>
          <div className="flex flex-wrap gap-4">
            <Button loading>Primary Loading</Button>
            <Button variant="outline" loading>Outline Loading</Button>
            <Button variant="secondary" size="small" loading>Small Loading</Button>
          </div>
        </div>

        {/* Disabled States */}
        <div className="mb-8">
          <h2 className="text-xl font-semibold text-gray-800 mb-4">Disabled States</h2>
          <div className="flex flex-wrap gap-4">
            <Button disabled>Primary Disabled</Button>
            <Button variant="outline" disabled>Outline Disabled</Button>
            <Button variant="secondary" disabled>Secondary Disabled</Button>
          </div>
        </div>

        {/* Combined Examples */}
        <div className="mb-8">
          <h2 className="text-xl font-semibold text-gray-800 mb-4">Combined Examples</h2>
          <div className="flex flex-wrap gap-4">
            <Button variant="primary" size="large">Large Primary</Button>
            <Button variant="outline" size="small" disabled>Small Outline Disabled</Button>
            <Button variant="secondary" loading>Secondary Loading</Button>
          </div>
        </div>

        {/* With Icons and Custom Content */}
        <div className="mb-8">
          <h2 className="text-xl font-semibold text-gray-800 mb-4">Custom Content</h2>
          <div className="flex flex-wrap gap-4">
            <Button>
              <svg className="mr-2 h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
              </svg>
              With Icon
            </Button>
            <Button variant="outline">
              Only Text
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
};

export default ButtonExamples;