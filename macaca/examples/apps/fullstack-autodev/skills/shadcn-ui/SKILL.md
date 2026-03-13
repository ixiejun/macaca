---
name: shadcn-ui
description: Comprehensive documentation for the shadcn/ui component library with CLI guidance, framework-specific setups, and practical implementation examples for Next.js, React, and TypeScript projects.
---
# shadcn/ui Component Library

Complete reference for building UIs with shadcn/ui — a collection of re-usable components built with Radix UI and Tailwind CSS.

## Quick Start (Next.js)

```bash
npx shadcn@latest init
```

Configuration prompts:
- Style: New York (recommended)
- Base color: Neutral
- CSS variables: Yes

## Adding Components

```bash
npx shadcn@latest add button
npx shadcn@latest add dialog
npx shadcn@latest add card
npx shadcn@latest add table
npx shadcn@latest add form
npx shadcn@latest add input
npx shadcn@latest add select
npx shadcn@latest add tabs
npx shadcn@latest add toast
npx shadcn@latest add dropdown-menu
```

## Component Usage Patterns

### Button

```tsx
import { Button } from "@/components/ui/button"

<Button variant="default">Primary</Button>
<Button variant="secondary">Secondary</Button>
<Button variant="destructive">Delete</Button>
<Button variant="outline">Outline</Button>
<Button variant="ghost">Ghost</Button>
<Button variant="link">Link</Button>
<Button size="sm">Small</Button>
<Button size="lg">Large</Button>
<Button disabled>Disabled</Button>
```

### Card

```tsx
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card"

<Card>
  <CardHeader>
    <CardTitle>Title</CardTitle>
    <CardDescription>Description</CardDescription>
  </CardHeader>
  <CardContent>
    <p>Content here</p>
  </CardContent>
  <CardFooter>
    <Button>Action</Button>
  </CardFooter>
</Card>
```

### Dialog

```tsx
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog"

<Dialog>
  <DialogTrigger asChild>
    <Button variant="outline">Open</Button>
  </DialogTrigger>
  <DialogContent>
    <DialogHeader>
      <DialogTitle>Title</DialogTitle>
      <DialogDescription>Description</DialogDescription>
    </DialogHeader>
    <div>Content</div>
    <DialogFooter>
      <Button type="submit">Save</Button>
    </DialogFooter>
  </DialogContent>
</Dialog>
```

### Form (React Hook Form + Zod)

```tsx
import { zodResolver } from "@hookform/resolvers/zod"
import { useForm } from "react-hook-form"
import * as z from "zod"
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage } from "@/components/ui/form"

const formSchema = z.object({
  username: z.string().min(2).max(50),
  email: z.string().email(),
})

function ProfileForm() {
  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    defaultValues: { username: "", email: "" },
  })

  function onSubmit(values: z.infer<typeof formSchema>) {
    console.log(values)
  }

  return (
    <Form {...form}>
      <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
        <FormField
          control={form.control}
          name="username"
          render={({ field }) => (
            <FormItem>
              <FormLabel>Username</FormLabel>
              <FormControl>
                <Input placeholder="username" {...field} />
              </FormControl>
              <FormMessage />
            </FormItem>
          )}
        />
        <Button type="submit">Submit</Button>
      </form>
    </Form>
  )
}
```

### Data Table

```tsx
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"

<Table>
  <TableHeader>
    <TableRow>
      <TableHead>Name</TableHead>
      <TableHead>Status</TableHead>
      <TableHead className="text-right">Amount</TableHead>
    </TableRow>
  </TableHeader>
  <TableBody>
    {items.map((item) => (
      <TableRow key={item.id}>
        <TableCell>{item.name}</TableCell>
        <TableCell>{item.status}</TableCell>
        <TableCell className="text-right">{item.amount}</TableCell>
      </TableRow>
    ))}
  </TableBody>
</Table>
```

## Theming

### Dark Mode (Next.js)

```bash
npm install next-themes
```

```tsx
// app/layout.tsx
import { ThemeProvider } from "@/components/theme-provider"

export default function RootLayout({ children }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body>
        <ThemeProvider attribute="class" defaultTheme="system" enableSystem>
          {children}
        </ThemeProvider>
      </body>
    </html>
  )
}
```

### Custom Colors

Edit `globals.css` CSS variables:
```css
@layer base {
  :root {
    --background: 0 0% 100%;
    --foreground: 222.2 84% 4.9%;
    --primary: 222.2 47.4% 11.2%;
    --primary-foreground: 210 40% 98%;
  }
  .dark {
    --background: 222.2 84% 4.9%;
    --foreground: 210 40% 98%;
  }
}
```

## Layout Patterns

### Sidebar Layout

```tsx
<div className="flex min-h-screen">
  <aside className="w-64 border-r bg-muted/40 p-4">
    <nav className="space-y-2">
      <Button variant="ghost" className="w-full justify-start">Dashboard</Button>
      <Button variant="ghost" className="w-full justify-start">Settings</Button>
    </nav>
  </aside>
  <main className="flex-1 p-6">{children}</main>
</div>
```

### Responsive Container

```tsx
<div className="container mx-auto px-4 py-8 max-w-7xl">
  {children}
</div>
```

## Best Practices

- Always use shadcn-ui components instead of raw HTML elements
- Prefer `variant` and `size` props over custom Tailwind overrides
- Use `cn()` utility for conditional class merging
- Install only the components you need (tree-shakeable by design)
- Use Zod + React Hook Form for all form validation
- Use CSS variables for theming, never hardcode colors
