import { spaces } from '@md/foundation'
import { PaginationState } from '@tanstack/react-table'
import { Flex, Skeleton } from '@ui/components'
import { useState } from 'react'

import { SubscriptionsTable } from '@/features/subscriptions/SubscriptionsTable'
import { useQuery } from '@/lib/connectrpc'
import { Customer } from '@/rpc/api/customers/v1/models_pb'
import { listSubscriptions } from '@/rpc/api/subscriptions/v1/subscriptions-SubscriptionsService_connectquery'

type Props = {
  customer: Customer
}

export const SubscriptionsCard = ({ customer }: Props) => {
  const [pagination, setPagination] = useState<PaginationState>({
    pageIndex: 0,
    pageSize: 20,
  })

  const invoicesQuery = useQuery(listSubscriptions, {
    pagination: {
      limit: pagination.pageSize,
      offset: pagination.pageIndex * pagination.pageSize,
    },
    customerId: customer.id,
  })

  return invoicesQuery.isLoading ? (
    <Flex direction="column" gap={spaces.space9} fullHeight>
      <Skeleton height={16} width={50} />
      <Skeleton height={44} />
    </Flex>
  ) : (
    <SubscriptionsTable
      data={invoicesQuery.data?.subscriptions || []}
      totalCount={invoicesQuery.data?.paginationMeta?.total || 0}
      pagination={pagination}
      setPagination={setPagination}
      isLoading={invoicesQuery.isLoading}
      hideCustomer
    />
  )
}
