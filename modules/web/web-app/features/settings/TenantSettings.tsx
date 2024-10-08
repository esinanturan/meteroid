import { Tabs, TabsContent, TabsList, TabsTrigger } from '@md/ui'
import { FunctionComponent } from 'react'

import { CompanyTab } from '@/features/settings/tabs/CompanyTab'
import { GeneralTab } from '@/features/settings/tabs/GeneralTab'
import { InvoiceTab } from '@/features/settings/tabs/InvoiceTab'
import { UsersTab } from '@/features/settings/tabs/UsersTab'
import { useQueryState } from '@/hooks/useQueryState'

export const TenantSettings: FunctionComponent = () => {
  const [tab, setTab] = useQueryState('tab', 'general')

  return (
    <>
      <div className="  space-y-6 w-full h-full overflow-x-hidden">
        <Tabs defaultValue={tab} onValueChange={setTab} className="w-full ">
          <TabsList className="w-full justify-start">
            <TabsTrigger value="general">General</TabsTrigger>
            <TabsTrigger value="merchant">Merchant</TabsTrigger>
            <TabsTrigger value="invoices">Invoices</TabsTrigger>
            <TabsTrigger value="integrations">Integrations</TabsTrigger>
            <TabsTrigger value="taxes">Taxes</TabsTrigger>
            <TabsTrigger value="users">Users</TabsTrigger>
          </TabsList>
          <TabsContent value="general">
            <GeneralTab />
          </TabsContent>
          <TabsContent value="merchant">
            <CompanyTab />
          </TabsContent>
          <TabsContent value="invoices">
            <InvoiceTab />
          </TabsContent>
          <TabsContent value="integrations">Not implemented</TabsContent>
          <TabsContent value="taxes">Not implemented</TabsContent>
          <TabsContent value="users">
            <UsersTab />
          </TabsContent>
        </Tabs>
      </div>
    </>
  )
}
