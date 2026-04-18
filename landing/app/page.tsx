import { SiteHeader } from "@/components/landing/site-header"
import { Hero } from "@/components/landing/hero"
import { Features } from "@/components/landing/features"
import { Models } from "@/components/landing/models"
import { Integrations } from "@/components/landing/integrations"
import { Workflow } from "@/components/landing/workflow"
import { Testimonials } from "@/components/landing/testimonials"
import { Pricing } from "@/components/landing/pricing"
import { CTA } from "@/components/landing/cta"
import { SiteFooter } from "@/components/landing/site-footer"

export default function Page() {
  return (
    <div className="min-h-screen bg-background text-foreground">
      <SiteHeader />
      <main>
        <Hero />
        <div aria-hidden className="border-t border-border/60" />
        <Features />
        <Models />
        <Integrations />
        <Workflow />
        <Testimonials />
        <Pricing />
        <CTA />
      </main>
      <SiteFooter />
    </div>
  )
}
