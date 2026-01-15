#!/usr/bin/env python3
"""
Import invoice data from CSV into Memory Graph MCP Server.
Parses Vietnamese invoice data and creates Customer + Invoice entities with relations.
Invoice CSV expected columns (in Vietnamese) from Odoo export:
- Số
- Hiển thị tên đối tác ở hóa đơn
- Ngày hóa đơn
- Ngày phải trả
- Nhân viên kinh doanh
- Tổng chưa thuế đã ký
- Thuế đã ký
- Tổng số đã ký
- Số tiền ký kết
- Tình trạng thanh toán
- Trạng thái
"""

import csv
import json
import requests
from datetime import datetime
from collections import defaultdict

# MCP Server endpoint
MCP_URL = "http://localhost:3030"

def parse_amount(amount_str: str) -> float:
    """Parse Vietnamese formatted amount string to float."""
    if not amount_str or amount_str.strip() == "":
        return 0.0
    # Remove quotes, commas, and convert to float
    cleaned = amount_str.replace('"', '').replace(',', '').strip()
    try:
        return float(cleaned)
    except ValueError:
        return 0.0

def format_vnd(amount: float) -> str:
    """Format amount as Vietnamese Dong."""
    if amount >= 1_000_000_000:
        return f"{amount/1_000_000_000:.2f} tỷ VND"
    elif amount >= 1_000_000:
        return f"{amount/1_000_000:.1f} triệu VND"
    else:
        return f"{amount:,.0f} VND"

def call_mcp_tool(tool_name: str, params: dict) -> dict:
    """Call MCP tool via HTTP endpoint."""
    payload = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": params
        }
    }
    try:
        response = requests.post(f"{MCP_URL}/mcp", json=payload, timeout=30)
        return response.json()
    except Exception as e:
        print(f"Error calling {tool_name}: {e}")
        return {"error": str(e)}

def load_csv(file_path: str) -> list:
    """Load and parse CSV file."""
    invoices = []
    with open(file_path, 'r', encoding='utf-8-sig') as f:  # utf-8-sig handles BOM
        reader = csv.DictReader(f)
        for row in reader:
            invoices.append({
                'invoice_no': row['Số'],
                'customer': row['Hiển thị tên đối tác ở hóa đơn'],
                'invoice_date': row['Ngày hóa đơn'],
                'due_date': row['Ngày phải trả'],
                'salesperson': row['Nhân viên kinh doanh'],
                'subtotal': parse_amount(row['Tổng chưa thuế đã ký']),
                'tax': parse_amount(row['Thuế đã ký']),
                'total': parse_amount(row['Tổng số đã ký']),
                'amount_due': parse_amount(row['Số tiền ký kết']),
                'payment_status': row['Tình trạng thanh toán'],
                'status': row['Trạng thái']
            })
    return invoices

def analyze_data(invoices: list) -> dict:
    """Analyze invoice data to extract customers and statistics."""
    customers = defaultdict(lambda: {
        'invoice_count': 0,
        'total_amount': 0.0,
        'unpaid_amount': 0.0,
        'paid_count': 0,
        'unpaid_count': 0,
        'salesperson': None,
        'first_invoice': None,
        'last_invoice': None,
        'overdue_count': 0
    })

    today = datetime.now().date()

    for inv in invoices:
        cust = inv['customer']
        customers[cust]['invoice_count'] += 1
        customers[cust]['total_amount'] += inv['total']
        customers[cust]['salesperson'] = inv['salesperson']

        # Track dates
        inv_date = inv['invoice_date']
        if customers[cust]['first_invoice'] is None or inv_date < customers[cust]['first_invoice']:
            customers[cust]['first_invoice'] = inv_date
        if customers[cust]['last_invoice'] is None or inv_date > customers[cust]['last_invoice']:
            customers[cust]['last_invoice'] = inv_date

        # Payment status
        if inv['payment_status'] == 'Chưa trả':
            customers[cust]['unpaid_count'] += 1
            customers[cust]['unpaid_amount'] += inv['amount_due']

            # Check overdue
            try:
                due_date = datetime.strptime(inv['due_date'], '%Y-%m-%d').date()
                if due_date < today:
                    customers[cust]['overdue_count'] += 1
            except:
                pass
        else:
            customers[cust]['paid_count'] += 1

    return dict(customers)

def create_customer_entities(customers: dict) -> int:
    """Create customer entities in Memory Graph."""
    entities = []

    for name, stats in customers.items():
        observations = [
            f"Tổng số hóa đơn: {stats['invoice_count']}",
            f"Tổng giá trị: {format_vnd(stats['total_amount'])}",
            f"Đã thanh toán: {stats['paid_count']} hóa đơn",
            f"Chưa thanh toán: {stats['unpaid_count']} hóa đơn",
        ]

        if stats['unpaid_amount'] > 0:
            observations.append(f"Công nợ hiện tại: {format_vnd(stats['unpaid_amount'])}")

        if stats['overdue_count'] > 0:
            observations.append(f"⚠️ Quá hạn: {stats['overdue_count']} hóa đơn")

        if stats['salesperson']:
            observations.append(f"Nhân viên phụ trách: {stats['salesperson']}")

        observations.append(f"Giao dịch từ: {stats['first_invoice']} đến {stats['last_invoice']}")

        entities.append({
            "name": name,
            "entityType": "Customer",
            "observations": observations
        })

    # Create in batches of 50
    batch_size = 50
    created = 0
    for i in range(0, len(entities), batch_size):
        batch = entities[i:i+batch_size]
        result = call_mcp_tool("create_entities", {"entities": batch})
        if "error" not in result:
            created += len(batch)
            print(f"  Created customers {i+1}-{min(i+batch_size, len(entities))} of {len(entities)}")

    return created

def create_invoice_entities(invoices: list, limit: int = None) -> int:
    """Create invoice entities in Memory Graph."""
    if limit:
        invoices = invoices[:limit]

    entities = []
    today = datetime.now().date()

    for inv in invoices:
        observations = [
            f"Khách hàng: {inv['customer']}",
            f"Ngày hóa đơn: {inv['invoice_date']}",
            f"Hạn thanh toán: {inv['due_date']}",
            f"Tổng tiền: {format_vnd(inv['total'])}",
            f"Thuế: {format_vnd(inv['tax'])}",
            f"Trạng thái: {inv['payment_status']}",
        ]

        if inv['payment_status'] == 'Chưa trả':
            try:
                due_date = datetime.strptime(inv['due_date'], '%Y-%m-%d').date()
                if due_date < today:
                    days_overdue = (today - due_date).days
                    observations.append(f"⚠️ QUÁ HẠN {days_overdue} ngày")
            except:
                pass
            observations.append(f"Số tiền còn nợ: {format_vnd(inv['amount_due'])}")

        if inv['salesperson']:
            observations.append(f"Nhân viên: {inv['salesperson']}")

        entities.append({
            "name": inv['invoice_no'],
            "entityType": "Invoice",
            "observations": observations
        })

    # Create in batches of 50
    batch_size = 50
    created = 0
    for i in range(0, len(entities), batch_size):
        batch = entities[i:i+batch_size]
        result = call_mcp_tool("create_entities", {"entities": batch})
        if "error" not in result:
            created += len(batch)
            print(f"  Created invoices {i+1}-{min(i+batch_size, len(entities))} of {len(entities)}")

    return created

def create_relations(invoices: list, customers: dict, limit: int = None) -> int:
    """Create relations between invoices and customers."""
    if limit:
        invoices = invoices[:limit]

    relations = []

    # Invoice -> Customer relations
    for inv in invoices:
        relations.append({
            "from": inv['invoice_no'],
            "to": inv['customer'],
            "relationType": "issued_to"
        })

    # Salesperson -> Customer relations (unique)
    for cust_name, stats in customers.items():
        if stats['salesperson']:
            relations.append({
                "from": stats['salesperson'],
                "to": cust_name,
                "relationType": "manages"
            })

    # Create in batches of 100
    batch_size = 100
    created = 0
    for i in range(0, len(relations), batch_size):
        batch = relations[i:i+batch_size]
        result = call_mcp_tool("create_relations", {"relations": batch})
        if "error" not in result:
            created += len(batch)
            print(f"  Created relations {i+1}-{min(i+batch_size, len(relations))} of {len(relations)}")

    return created

def main():
    print("=" * 60)
    print("Memory Graph - Invoice Data Import")
    print("=" * 60)

    # Load CSV
    csv_path = r"export_invoices_from_odoo.csv"
    print(f"\n📂 Loading CSV: {csv_path}")
    invoices = load_csv(csv_path)
    print(f"   Found {len(invoices)} invoices")

    # Analyze data
    print("\n📊 Analyzing data...")
    customers = analyze_data(invoices)
    print(f"   Found {len(customers)} unique customers")

    # Summary statistics
    total_amount = sum(c['total_amount'] for c in customers.values())
    total_unpaid = sum(c['unpaid_amount'] for c in customers.values())
    total_overdue = sum(c['overdue_count'] for c in customers.values())

    print(f"\n📈 Summary:")
    print(f"   Total invoices: {len(invoices)}")
    print(f"   Total customers: {len(customers)}")
    print(f"   Total value: {format_vnd(total_amount)}")
    print(f"   Unpaid amount: {format_vnd(total_unpaid)}")
    print(f"   Overdue invoices: {total_overdue}")

    # Create entities
    print("\n🔧 Creating Customer entities...")
    cust_created = create_customer_entities(customers)
    print(f"   ✅ Created {cust_created} customers")

    print("\n🔧 Creating Invoice entities...")
    inv_created = create_invoice_entities(invoices)
    print(f"   ✅ Created {inv_created} invoices")

    print("\n🔧 Creating Relations...")
    rel_created = create_relations(invoices, customers)
    print(f"   ✅ Created {rel_created} relations")

    print("\n" + "=" * 60)
    print("✅ Import completed!")
    print(f"   Entities: {cust_created + inv_created}")
    print(f"   Relations: {rel_created}")
    print("=" * 60)

    # Test search
    print("\n🔍 Testing search...")
    result = call_mcp_tool("search_nodes", {"query": "quá hạn", "limit": 5})
    if "result" in result:
        content = result.get("result", {}).get("content", [])
        if content:
            data = json.loads(content[0].get("text", "{}"))
            entities = data.get("entities", [])
            print(f"   Found {len(entities)} entities matching 'quá hạn'")
            for e in entities[:3]:
                print(f"     - {e['name']}: {e['entityType']}")

if __name__ == "__main__":
    main()
