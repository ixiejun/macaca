import React from 'react';
import { Button } from '@/components/ui/button';

/**
 * Button 组件使用示例
 * 
 * 这个页面展示了Button组件的各种变体和尺寸组合
 */
export default function ButtonExamples() {
  const handleClick = () => {
    console.log('Button clicked!');
  };

  return (
    <div className="p-8 max-w-4xl mx-auto">
      <h1 className="text-3xl font-bold mb-8 text-gray-900">Button Component Examples</h1>
      
      {/* 变体示例 */}
      <div className="mb-12">
        <h2 className="text-xl font-semibold mb-4 text-gray-800">Variants</h2>
        <div className="flex flex-wrap gap-4 items-center">
          <Button variant="primary" onClick={handleClick}>
            Primary
          </Button>
          <Button variant="secondary" onClick={handleClick}>
            Secondary
          </Button>
          <Button variant="outline" onClick={handleClick}>
            Outline
          </Button>
          <Button variant="ghost" onClick={handleClick}>
            Ghost
          </Button>
          <Button variant="destructive" onClick={handleClick}>
            Destructive
          </Button>
          <Button variant="link" onClick={handleClick}>
            Link
          </Button>
        </div>
      </div>

      {/* 尺寸示例 */}
      <div className="mb-12">
        <h2 className="text-xl font-semibold mb-4 text-gray-800">Sizes</h2>
        <div className="flex flex-wrap gap-4 items-center">
          <Button size="sm" onClick={handleClick}>
            Small
          </Button>
          <Button size="md" onClick={handleClick}>
            Medium
          </Button>
          <Button size="lg" onClick={handleClick}>
            Large
          </Button>
          <Button size="icon" onClick={handleClick}>
            👍
          </Button>
        </div>
      </div>

      {/* 禁用状态示例 */}
      <div className="mb-12">
        <h2 className="text-xl font-semibold mb-4 text-gray-800">Disabled States</h2>
        <div className="flex flex-wrap gap-4 items-center">
          <Button variant="primary" disabled>
            Primary Disabled
          </Button>
          <Button variant="outline" disabled>
            Outline Disabled
          </Button>
          <Button variant="destructive" disabled>
            Destructive Disabled
          </Button>
        </div>
      </div>

      {/* 组合示例 */}
      <div className="mb-12">
        <h2 className="text-xl font-semibold mb-4 text-gray-800">Variant + Size Combinations</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          <div>
            <h3 className="text-sm font-medium mb-2 text-gray-700">Primary Variants</h3>
            <div className="flex flex-col gap-2">
              <Button variant="primary" size="sm">Small Primary</Button>
              <Button variant="primary" size="md">Medium Primary</Button>
              <Button variant="primary" size="lg">Large Primary</Button>
            </div>
          </div>
          
          <div>
            <h3 className="text-sm font-medium mb-2 text-gray-700">Outline Variants</h3>
            <div className="flex flex-col gap-2">
              <Button variant="outline" size="sm">Small Outline</Button>
              <Button variant="outline" size="md">Medium Outline</Button>
              <Button variant="outline" size="lg">Large Outline</Button>
            </div>
          </div>
          
          <div>
            <h3 className="text-sm font-medium mb-2 text-gray-700">Destructive Variants</h3>
            <div className="flex flex-col gap-2">
              <Button variant="destructive" size="sm">Small Destructive</Button>
              <Button variant="destructive" size="md">Medium Destructive</Button>
              <Button variant="destructive" size="lg">Large Destructive</Button>
            </div>
          </div>
        </div>
      </div>

      {/* 带图标的按钮示例 */}
      <div className="mb-12">
        <h2 className="text-xl font-semibold mb-4 text-gray-800">Buttons with Icons</h2>
        <div className="flex flex-wrap gap-4 items-center">
          <Button variant="primary">
            <span className="mr-2">🔍</span>
            Search
          </Button>
          <Button variant="outline">
            Save
            <span className="ml-2">💾</span>
          </Button>
          <Button variant="ghost" size="icon">
            ⭐
          </Button>
          <Button variant="destructive" size="icon">
            🗑️
          </Button>
        </div>
      </div>
    </div>
  );
}